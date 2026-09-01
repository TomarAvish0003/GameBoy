import init, * as wasm from "./pkg/wasm.js";

const WIDTH = 160;
const HEIGHT = 144;
const SCALE = 3;

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

class GameBoyAudio {
    constructor(sampleRate = 44100) {
        this.sampleRate = sampleRate;
        this.audioCtx = null;
        this.scriptNode = null;
        this.audioQueue = [];
    }

    init() {
        if (this.audioCtx) return;
        const AudioContextClass = window.AudioContext || window.webkitAudioContext;
        this.audioCtx = new AudioContextClass({ sampleRate: this.sampleRate });

        const bufferSize = 2048;
        this.scriptNode = this.audioCtx.createScriptProcessor(bufferSize, 0, 2);

        this.scriptNode.onaudioprocess = (e) => {
            const left = e.outputBuffer.getChannelData(0);
            const right = e.outputBuffer.getChannelData(1);

            for (let i = 0; i < bufferSize; i++) {
                if (this.audioQueue.length >= 2) {
                    left[i] = this.audioQueue.shift();
                    right[i] = this.audioQueue.shift();
                } else {
                    left[i] = 0.0;
                    right[i] = 0.0;
                }
            }
        };

        this.scriptNode.connect(this.audioCtx.destination);
    }

    resume() {
        if (!this.audioCtx) {
            this.init();
        }
        if (this.audioCtx.state === "suspended") {
            this.audioCtx.resume();
        }
    }

    queueSamples(samples) {
        if (!this.audioCtx || this.audioCtx.state !== "running") return;

        // Cap queue to 100ms to prevent latency build-up
        const MAX_QUEUE = this.sampleRate * 2 * 0.1;
        if (this.audioQueue.length > MAX_QUEUE) {
            this.audioQueue.length = 0;
        }

        for (let i = 0; i < samples.length; i++) {
            this.audioQueue.push(samples[i]);
        }
    }
}

async function run() {
    await init();
    const gb = new wasm.GB();
    const audio = new GameBoyAudio(44100);

    // Browser audio policy requires user gesture unlock
    window.addEventListener("click", () => audio.resume(), { once: true });
    window.addEventListener("keydown", () => audio.resume(), { once: true });

    const fileInput = document.getElementById("fileinput");

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

            function loop() {
                // 1. Advance emulation by 1 frame
                gb.step_frame();

                // 2. Fetch and queue audio
                const samples = gb.get_audio_samples();
                if (samples.length > 0) {
                    audio.queueSamples(samples);
                }

                // 3. Render video frame
                const screenBuffer = gb.get_screen();
                const clamped = new Uint8ClampedArray(screenBuffer);
                const imgData = new ImageData(clamped, WIDTH, HEIGHT);

                offscreenCtx.putImageData(imgData, 0, 0);
                ctx.drawImage(offscreenCanvas, 0, 0, canvas.width, canvas.height);

                animFrameId = requestAnimationFrame(loop);
            }

            animFrameId = requestAnimationFrame(loop);
        };

        reader.readAsArrayBuffer(file);
    });

    window.addEventListener("keydown", (e) => gb.press_button(e.key, true));
    window.addEventListener("keyup", (e) => gb.press_button(e.key, false));
}

run().catch(console.error);