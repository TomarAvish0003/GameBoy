import init, { GB } from "./pkg/wasm.js";
import { AudioManager } from "./audio.js";

const WIDTH = 160;
const HEIGHT = 144;
const SCALE = 3;

// Authentic Game Boy Frame Timing: 4,194,304 Hz / 70,224 cycles = 59.7275 FPS
const GB_FRAME_TIME_MS = 1000 / 59.7275; // ~16.7438 ms

// Palettes: [Darkest, Dark, Light, Lightest] as RGBA [r, g, b, a]
const PALETTES = {
    peagreen: [
        [15, 56, 15, 255],
        [48, 98, 48, 255],
        [139, 172, 15, 255],
        [155, 188, 15, 255]
    ],
    pocket: [
        [20, 20, 20, 255],
        [86, 86, 86, 255],
        [160, 160, 160, 255],
        [230, 230, 230, 255]
    ],
    oled: [
        [0, 0, 0, 255],
        [60, 60, 60, 255],
        [170, 170, 170, 255],
        [255, 255, 255, 255]
    ]
};

let currentPalette = "peagreen";

// Key mappings (maps KeyboardEvent.code to GB button index)
// Indices: 0: Right, 1: Left, 2: Up, 3: Down, 4: A, 5: B, 6: Select, 7: Start
const DEFAULT_KEYBINDS = {
    ArrowRight: 0,
    ArrowLeft: 1,
    ArrowUp: 2,
    ArrowDown: 3,
    KeyZ: 4,       // A Button
    KeyX: 5,       // B Button
    ShiftRight: 6, // Select
    ShiftLeft: 6,  // Select (alt)
    Enter: 7       // Start
};

let keybinds = { ...DEFAULT_KEYBINDS };
let listeningForButton = null;

const savedBinds = localStorage.getItem("gb_keybinds");
if (savedBinds) {
    try { keybinds = JSON.parse(savedBinds); } catch (_) {}
}

const canvas = document.getElementById("canvas");
canvas.width = WIDTH * SCALE;
canvas.height = HEIGHT * SCALE;

const ctx = canvas.getContext("2d");
ctx.imageSmoothingEnabled = false;

const offscreenCanvas = document.createElement("canvas");
offscreenCanvas.width = WIDTH;
offscreenCanvas.height = HEIGHT;
const offscreenCtx = offscreenCanvas.getContext("2d");

let animFrameId = null;
let gbInstance = null;

// Gamepad polling state
let prevGamepadState = {};

function pollGamepad(gb) {
    const gamepads = navigator.getGamepads ? navigator.getGamepads() : [];
    const gp = gamepads[0];
    if (!gp) return;

    const mappings = [
        { btn: 15, idx: 0 }, // D-Pad Right
        { btn: 14, idx: 1 }, // D-Pad Left
        { btn: 12, idx: 2 }, // D-Pad Up
        { btn: 13, idx: 3 }, // D-Pad Down
        { btn: 1,  idx: 4 }, // Gamepad B -> A
        { btn: 0,  idx: 5 }, // Gamepad A -> B
        { btn: 8,  idx: 6 }, // Select
        { btn: 9,  idx: 7 }  // Start
    ];

    for (const { btn, idx } of mappings) {
        const isPressed = gp.buttons[btn] && gp.buttons[btn].pressed;
        if (prevGamepadState[idx] !== isPressed) {
            gb.press_button(idx, isPressed);
            prevGamepadState[idx] = isPressed;
        }
    }
}

function applyPalette(rawBuffer, targetBuffer, paletteName) {
    const pal = PALETTES[paletteName] || PALETTES.peagreen;
    const totalPixels = WIDTH * HEIGHT;

    for (let i = 0; i < totalPixels; i++) {
        const srcIdx = i * 4;
        const r = rawBuffer[srcIdx];
        const g = rawBuffer[srcIdx + 1];
        const b = rawBuffer[srcIdx + 2];

        const lum = (r * 0.299 + g * 0.587 + b * 0.114) | 0;
        const shade = lum < 64 ? 0 : lum < 128 ? 1 : lum < 192 ? 2 : 3;

        const color = pal[shade];
        targetBuffer[srcIdx] = color[0];
        targetBuffer[srcIdx + 1] = color[1];
        targetBuffer[srcIdx + 2] = color[2];
        targetBuffer[srcIdx + 3] = 255;
    }
}

export function updateKeybindBadges() {
    const badgeMap = {
        0: "badge-right",
        1: "badge-left",
        2: "badge-up",
        3: "badge-down",
        4: "badge-a",
        5: "badge-b",
        6: "badge-select",
        7: "badge-start"
    };

    const invMap = {};
    for (const [code, idx] of Object.entries(keybinds)) {
        invMap[idx] = code.replace("Key", "").replace("Arrow", "");
    }

    for (const [idx, elemId] of Object.entries(badgeMap)) {
        const badge = document.getElementById(elemId);
        if (badge) {
            badge.innerText = invMap[idx] || "--";
        }
    }
}

export function startRebind(btnIdx, buttonElement) {
    listeningForButton = btnIdx;
    if (buttonElement) {
        buttonElement.classList.add("binding-active");
    }
}

async function run() {
    // init() returns the WebAssembly exports object containing .memory
    const wasm = await init("./pkg/wasm_bg.wasm");
    const gb = new GB();
    gbInstance = gb;

    const audio = new AudioManager();

    const unlockAudio = () => {
        audio.init();
        window.removeEventListener("click", unlockAudio);
        window.removeEventListener("keydown", unlockAudio);
    };
    window.addEventListener("click", unlockAudio);
    window.addEventListener("keydown", unlockAudio);

    updateKeybindBadges();

    // Drawer Setup
    const drawer = document.getElementById("drawer");
    const drawerToggle = document.getElementById("drawer-toggle");
    if (drawer && drawerToggle) {
        drawerToggle.addEventListener("click", () => {
            drawer.classList.toggle("open");
        });
    }

    // Palette Selection
    document.querySelectorAll('input[name="palette"]').forEach((radio) => {
        radio.addEventListener("change", (e) => {
            if (PALETTES[e.target.value]) {
                currentPalette = e.target.value;
            }
        });
    });

    // Volume Slider
    const volumeSlider = document.getElementById("volume-slider");
    if (volumeSlider) {
        volumeSlider.addEventListener("input", (e) => {
            audio.setVolume(parseFloat(e.target.value));
        });
    }

    // Channel Muting
    const ch1Toggle = document.getElementById("ch1-toggle");
    const ch2Toggle = document.getElementById("ch2-toggle");
    const ch3Toggle = document.getElementById("ch3-toggle");
    const ch4Toggle = document.getElementById("ch4-toggle");

    if (ch1Toggle) ch1Toggle.addEventListener("change", (e) => gb.set_channel_enabled(1, e.target.checked));
    if (ch2Toggle) ch2Toggle.addEventListener("change", (e) => gb.set_channel_enabled(2, e.target.checked));
    if (ch3Toggle) ch3Toggle.addEventListener("change", (e) => gb.set_channel_enabled(3, e.target.checked));
    if (ch4Toggle) ch4Toggle.addEventListener("change", (e) => gb.set_channel_enabled(4, e.target.checked));

    // Rebind Button Listeners
    const chassisButtons = [
        { id: "btn-dpad-up", idx: 2 },
        { id: "btn-dpad-down", idx: 3 },
        { id: "btn-dpad-left", idx: 1 },
        { id: "btn-dpad-right", idx: 0 },
        { id: "btn-a", idx: 4 },
        { id: "btn-b", idx: 5 },
        { id: "btn-select", idx: 6 },
        { id: "btn-start", idx: 7 }
    ];

    chassisButtons.forEach(({ id, idx }) => {
        const elem = document.getElementById(id);
        if (elem) {
            elem.addEventListener("click", () => {
                document.querySelectorAll(".binding-active").forEach((b) => b.classList.remove("binding-active"));
                startRebind(idx, elem);
            });
        }
    });

    // ROM Loader & Emulation Loop
    const fileInput = document.getElementById("fileinput");
    if (fileInput) {
        fileInput.addEventListener("change", (e) => {
            audio.resume();

            if (animFrameId) {
                cancelAnimationFrame(animFrameId);
            }

            const file = e.target.files[0];
            if (!file) return;

            const reader = new FileReader();
            reader.onload = () => {
                const rom = new Uint8Array(reader.result);
                gb.load_rom(rom);

                const led = document.querySelector(".battery-led .led-light");
                if (led) {
                    led.classList.add("active");
                }

                const finalPixelData = new Uint8ClampedArray(WIDTH * HEIGHT * 4);
                const imgData = new ImageData(finalPixelData, WIDTH, HEIGHT);

                let lastTime = performance.now();
                let frameAccumulator = 0;
                let frameCounter = 0;
                let lastFpsUpdate = performance.now();
                const fpsElem = document.getElementById("fps-display");

                function loop(now) {
                    pollGamepad(gb);

                    let delta = now - lastTime;
                    lastTime = now;

                    if (delta > 100) delta = 100;
                    frameAccumulator += delta;

                    let stepsRun = 0;
                    while (frameAccumulator >= GB_FRAME_TIME_MS) {
                        gb.step_frame();
                        frameAccumulator -= GB_FRAME_TIME_MS;
                        stepsRun++;
                        frameCounter++;

                        const samples = gb.get_audio_samples();
                        if (samples.length > 0) {
                            audio.pushSamples(samples);
                        }

                        if (stepsRun >= 4) {
                            frameAccumulator = 0;
                            break;
                        }
                    }

                    if (stepsRun > 0) {
                        const screenPtr = gb.get_screen_ptr();
                        // Access memory directly from the initialized WebAssembly exports
                        const rawWasmPixels = new Uint8Array(wasm.memory.buffer, screenPtr, WIDTH * HEIGHT * 4);

                        applyPalette(rawWasmPixels, finalPixelData, currentPalette);

                        offscreenCtx.putImageData(imgData, 0, 0);
                        ctx.drawImage(offscreenCanvas, 0, 0, canvas.width, canvas.height);
                    }

                    const elapsedFps = now - lastFpsUpdate;
                    if (elapsedFps >= 500) {
                        const currentFps = (frameCounter * 1000) / elapsedFps;
                        const speedPercent = ((currentFps / 59.7275) * 100).toFixed(0);
                        if (fpsElem) {
                            fpsElem.innerText = `${currentFps.toFixed(1)} FPS (${speedPercent}%)`;
                            fpsElem.style.color = (speedPercent >= 97 && speedPercent <= 103) ? "#39ff14" : "#ffb703";
                        }
                        frameCounter = 0;
                        lastFpsUpdate = now;
                    }

                    animFrameId = requestAnimationFrame(loop);
                }

                lastTime = performance.now();
                lastFpsUpdate = performance.now();
                frameAccumulator = 0;
                frameCounter = 0;
                animFrameId = requestAnimationFrame(loop);
            };

            reader.readAsArrayBuffer(file);
        });
    }

    // Keyboard Input Listeners
    window.addEventListener("keydown", (e) => {
        audio.resume();

        if (listeningForButton !== null) {
            e.preventDefault();
            for (const [code, bIdx] of Object.entries(keybinds)) {
                if (bIdx === listeningForButton) {
                    delete keybinds[code];
                }
            }
            keybinds[e.code] = listeningForButton;
            localStorage.setItem("gb_keybinds", JSON.stringify(keybinds));

            document.querySelectorAll(".binding-active").forEach((b) => b.classList.remove("binding-active"));
            listeningForButton = null;
            updateKeybindBadges();
            return;
        }

        if (keybinds[e.code] !== undefined) {
            e.preventDefault();
            gb.press_button(keybinds[e.code], true);
        }
    });

    window.addEventListener("keyup", (e) => {
        if (keybinds[e.code] !== undefined) {
            e.preventDefault();
            gb.press_button(keybinds[e.code], false);
        }
    });

    window.setEmulatorPalette = (name) => {
        if (PALETTES[name]) currentPalette = name;
    };

    window.setChannelEnabled = (channel, enabled) => {
        if (gbInstance) gbInstance.set_channel_enabled(channel, enabled);
    };

    window.setMasterVolume = (vol) => {
        audio.setVolume(vol);
    };
}

run().catch(console.error);