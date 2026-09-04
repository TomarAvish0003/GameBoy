# Game Boy (DMG-01) Emulator in Rust

A cycle-accurate Nintendo Game Boy (DMG-01) emulator implemented from scratch in Rust. The project is structured as a decoupled multi-crate workspace featuring a platform-agnostic hardware emulation core, a hardware-accelerated desktop application with an immediate-mode GUI and interactive debugger, and a WebAssembly browser client powered by the Web Audio API.

---

## Table of Contents

- [Architecture Overview](#architecture-overview)
- [Technical Specifications & Subsystems](#technical-specifications--subsystems)
  - [Sharp SM83 CPU Core](#sharp-sm83-cpu-core)
  - [Pixel Processing Unit (PPU)](#pixel-processing-unit-ppu)
  - [Audio Processing Unit (APU)](#audio-processing-unit-apu)
  - [Memory Bank Controllers (MBC) & Storage](#memory-bank-controllers-mbc--storage)
  - [Timers & Interrupt Controller](#timers--interrupt-controller)
- [Desktop Client](#desktop-client)
- [WebAssembly Client](#webassembly-client)
- [Hardware Compliance & Test Suite](#hardware-compliance--test-suite)
- [Technology Stack](#technology-stack)
- [Repository Structure](#repository-structure)
- [Building and Running](#building-and-running)
  - [Prerequisites](#prerequisites)
  - [Desktop Application](#desktop-application)
  - [WebAssembly / Web Application](#webassembly--web-application)
  - [Running Test Suites](#running-test-suites)
- [Controls](#controls)
- [Debugger Reference](#debugger-reference)
- [License & Acknowledgments](#license--acknowledgments)

---

## Architecture Overview

The codebase is organized into distinct layers to enforce strict separation of concerns between emulation logic and platform-specific I/O:

```
+-------------------------------------------------------------------------+
|                                Frontends                                |
|                                                                         |
|   +---------------------------------+   +---------------------------+   |
|   |         Desktop Client          |   |       Web / WASM          |   |
|   |  - SDL2 / OpenGL 3.3 Core       |   |  - wasm-bindgen / web-sys |   |
|   |  - egui Immediate-Mode GUI      |   |  - HTML5 Canvas           |   |
|   |  - AudioQueue Driver            |   |  - AudioWorkletNode       |   |
|   |  - CLI Debugger (gbd)           |   |  - LocalStorage / Touch   |   |
|   +---------------------------------+   +---------------------------+   |
+------------------------------------+------------------------------------+
                                     |
                                     v
+-------------------------------------------------------------------------+
|                       Hardware Core (`gb_core`)                         |
|                                                                         |
|   +-------------------+  +-------------------+  +-------------------+   |
|   |     SM83 CPU      |  |    PPU Engine     |  |    APU Engine     |   |
|   |  - 4.194304 MHz   |  |  - Scanline Mode  |  |  - 4 Sound Chans  |   |
|   |  - T-Cycle Bus    |  |  - 40 Sprites     |  |  - Frame Seq      |   |
|   |  - Halt Bug / IME |  |  - STAT IRQs      |  |  - Wave RAM Quirk |   |
|   +-------------------+  +-------------------+  +-------------------+   |
|   +-------------------+  +-------------------+  +-------------------+   |
|   |   Memory / MBC    |  |   System Timer    |  |    Joypad / IO    |   |
|   |  - MBC1/2/3/5     |  |  - DIV / TIMA     |  |  - P14/P15 Matrix |   |
|   |  - RTC Latching   |  |  - Falling Edge   |  |  - Joypad IRQ     |   |
|   +-------------------+  +-------------------+  +-------------------+   |
|                                                                         |
|   Platform-agnostic, zero-OS dependencies, Serde-serializable state     |
+-------------------------------------------------------------------------+
```

1. **`gb_core`**: The standalone emulation library. It has zero native platform or windowing dependencies. All components step by discrete T-cycles (clock ticks at 4.194304 MHz). It exposes cleanly isolated APIs for frame stepping, raw pixel buffer access, audio sample generation, controller inputs, and deterministic state serialization via `serde`.
2. **`desktop`**: Native client leveraging SDL2 and modern OpenGL 3.3 Core Profile. Features an `egui` interface, dynamic frame pacing, audio stream synchronization, gamepad integration, and a non-blocking interactive CLI debugger.
3. **`wasm`**: WebAssembly binding interface providing zero-copy buffer views into the Game Boy's 160x144 framebuffer and audio streams, consumed by a pure JavaScript/HTML5 frontend.

---

## Technical Specifications & Subsystems

### Sharp SM83 CPU Core

The CPU is an 8-bit Sharp SM83 processor (hybrid architecture derived from Zilog Z80 and Intel 8080) running at 4.194304 MHz.

- **Cycle-Accurate Memory Bus Step**: Every instruction execution is broken down into machine cycles (M-cycles = 4 T-cycles). Bus read/write operations advance the system clock by 2 T-cycles before memory access and 2 T-cycles after, preserving cycle-level bus contention and synchronization across peripheral components.
- **Interrupt Controller**: Full reproduction of the five interrupt vectors with priority-based arbitration:
  - V-Blank (`$0040`)
  - LCD STAT (`$0048`)
  - Timer (`$0050`)
  - Serial (`$0058`)
  - Joypad (`$0060`)
- **EI Delay & IME Scheduling**: Immediate vs. scheduled Interrupt Master Enable (`IME`) toggling accurately delays interrupt servicing by one instruction following an `EI` execution.
- **HALT Bug**: Emulates the hardware defect where executing `HALT` with `IME = 0` and pending interrupts causes the instruction pointer (`PC`) to fail to increment on the subsequent opcode, resulting in byte-reuse glitches utilized by commercial games and test ROMs.

### Pixel Processing Unit (PPU)

The PPU outputs a 160x144 display at ~59.7275 Hz (70,224 clock cycles per frame) across 154 scanlines (144 active, 10 V-Blank).

- **Scanline State Machine**:
  - **Mode 2 (OAM Search)**: 80 T-cycles; inspects 40 sprite attributes in OAM (`$FE00–$FE9F`) and identifies up to 10 visible sprites per line.
  - **Mode 3 (Pixel Transfer)**: Variable 172 to 289 T-cycles depending on sprite density and SCX scroll offsets; renders background, window, and sprite layers.
  - **Mode 0 (H-Blank)**: 204 T-cycles remaining in the scanline; enables CPU access to VRAM and OAM.
  - **Mode 1 (V-Blank)**: Scanlines 144 through 153 (456 T-cycles each; 4560 T-cycles total).
- **STAT Interrupt Generation**: Cycle-exact trigger conditions for `LYC=LY` coincidence, Mode 0 (H-Blank), Mode 1 (V-Blank), and Mode 2 (OAM).
- **Window & Background**: Hardware scroll registers (`SCX`, `SCY`), window positioning (`WX`, `WY`) with internal window line counter resets, and tile addressing modes (`$8000–$8FFF` signed vs. `$8800–$97FF` unsigned).
- **Sprite Rendering (OAM)**: Supports both 8x8 and 8x16 sprite modes, horizontal/vertical coordinate mirroring (X/Y flip), object palette mapping (`OBP0`/`OBP1`), and priority masking over background tiles.
- **OAM DMA Controller**: Direct memory transfer (`$FF46`) copying 160 bytes from ROM/RAM to OAM over 160 µs (640 T-cycles).

### Audio Processing Unit (APU)

A 4-channel audio synthesizer that outputs stereo sound sampled at 44.1 kHz.

- **Channel 1 (Pulse with Sweep)**: 4 duty cycle profiles (12.5%, 25%, 50%, 75%), volume envelope generator, and 7-bit frequency sweep with overflow calculation and auto-disable.
- **Channel 2 (Pulse)**: Secondary square channel with independent volume envelope and duty cycle generation.
- **Channel 3 (Programmable Wave)**: 32 4-bit sample custom waveform playback from Wave RAM (`$FF30–$FF3F`).
  - **Hardware Quirk Accuracy**: Emulates the DMG Wave RAM access cycle quirk. Reads and writes while the channel is enabled only succeed during the sample fetch window; otherwise, reads return `$FF` and writes are dropped.
  - **Retrigger Glitch**: Accurately replicates the DMG wave corruption bug when retriggering Channel 3 within 1 cycle of a sample read (copying the active 4-byte block to `$FF30–$FF33`).
- **Channel 4 (White Noise)**: Linear Feedback Shift Register (LFSR) supporting both 15-bit standard pseudo-random noise and 7-bit metallic/short-period mode.
- **512 Hz Frame Sequencer**: Synchronized clocking divider advancing:
  - Step 0, 2, 4, 6: Length counters (256 Hz)
  - Step 2, 6: Frequency sweep (128 Hz)
  - Step 7: Volume envelope (64 Hz)
  - Accurately models odd-step extra length clocking glitches and power-cycle frame sequencer resets.
- **Compliance**: Passes 100% of all 12 test singles in Blargg's `dmg_sound` test suite.

### Memory Bank Controllers (MBC) & Storage

Modular cartridge bus implementation supporting cartridge headers, checksum validation, and expansion hardware:

- **ROM Only**: Cartridges up to 32 KB without external banking.
- **MBC1**: Up to 2 MB ROM and 32 KB RAM, supporting both ROM banking mode (`16Mbit ROM / 8KByte RAM`) and RAM banking mode (`4Mbit ROM / 32KByte RAM`).
- **MBC2**: Up to 256 KB ROM with 512x4-bit integrated internal RAM and address line control bits.
- **MBC3**: Up to 2 MB ROM and 32 KB RAM with complete Real-Time Clock (RTC) emulation:
  - Day counter, hours, minutes, seconds, sub-seconds.
  - RTC register latching mechanism (`0x00 -> 0x01` write sequence).
  - RTC halt flag and day counter overflow bit.
- **MBC5**: Up to 8 MB ROM with 9-bit ROM bank index addressing and 128 KB RAM banking.
- **Battery-Backed SRAM**: Dirty tracking detects cartridge RAM mutations and auto-persists `.sav` files to disk upon modified data.
- **Save States**: Comprehensive state snapshotting and restoration using `serde` and `bincode`.

### Timers & Interrupt Controller

- **Internal Divider (`DIV`)**: 16-bit internal clock counter clocked at 4.194304 MHz, where the upper 8 bits form `$FF04`.
- **Timer Counter (`TIMA`)**: Programmable timer clocked by selecting multiplexer bits from `DIV` via `TAC` (frequencies: 4096 Hz, 262144 Hz, 65536 Hz, 16384 Hz).
- **Edge Detector & Overflow Glitch**: Emulates falling-edge detection on DIV writes (triggering unexpected TIMA clocking) and the 1 M-cycle delayed interrupt reload cycle where intermediate writes can cancel or overwrite the reload state.

---

## Desktop Client

The native client (`desktop`) is built for high performance and low-latency interaction:

- **Rendering**: Hardware-accelerated OpenGL 3.3 Core profile rendering via `gl` and `sdl2`.
- **Display HUD**: Immediate-mode GUI built with `egui` (`egui_sdl2_gl`), featuring:
  - Scale factor adjustments (1x, 2x, 3x, 4x, 5x, Fullscreen).
  - Fixed-ratio aspect locking.
  - Authentic color palette selection (Classic DMG Green, Pocket B&W, Game Boy Light Teal, Warm Amber, High-Contrast OLED).
  - Optional LCD phosphor ghosting / persistence simulation.
- **Timing & Frame Pacer**:
  - Dynamically elevates the Windows kernel timer resolution to 1.0 ms via `timeBeginPeriod(1)`.
  - Software frame pacer precisely locks execution to 59.7275 FPS (16.7438 ms per frame).
  - Fast-forward support (unbounded or 2x/4x/8x multiplier).
- **Audio Output Driver**:
  - Streams interleaved 44.1 kHz stereo audio via SDL2 `AudioQueue<f32>`.
  - Dynamic audio queue pacing: subtly modulates emulation speed based on remaining audio queue samples to eliminate buffer underflow crackles and buffer overflow latency.
  - Individual channel mute switches (Ch1, Ch2, Ch3, Ch4) for audio inspection.
- **Save State Manager**: 9 quicksave slots with instant write/load shortcuts.
- **Controller Support**: Native gamepad support via SDL2 GameController API with hotplug detection.
- **Native File Dialogs**: Cross-platform file picker integration via `rfd`.

---

## WebAssembly Client

The web implementation (`wasm` + `html`) compiles the core into WebAssembly to run entirely in modern web browsers:

- **Zero-Copy Framebuffer**: Exposes raw pointer memory directly to the JavaScript runtime via `wasm-bindgen`, transferring 160x144 RGBA pixel data to an HTML5 canvas with minimal allocation overhead.
- **Multi-Threaded Web Audio**: Employs an `AudioWorkletNode` (`audio-processor.js`) running on a dedicated browser audio thread with a circular lockless ring buffer to maintain sample stability under UI load.
- **SRAM & State Storage**: Automatically syncs battery-backed save data and save state snapshots with browser `localStorage`.
- **Responsive Controls**: Virtual on-screen D-Pad and action buttons for mobile and touch devices, alongside keyboard controls.
- **ROM Drag-and-Drop**: Load arbitrary `.gb` files directly from the desktop filesystem.

---

## Hardware Compliance & Test Suite

The emulator has been tested extensively against standard industry hardware test suites:

### Blargg `dmg_sound` Test Suite (100% Pass)
| Test ROM | Component Verified | Result |
|---|---|:---:|
| `01-registers.gb` | Register read/write masks and power-cycle states | **PASSED** |
| `02-len ctr.gb` | Length counter clocking and enable behavior | **PASSED** |
| `03-trigger.gb` | Channel triggering and state re-initialization | **PASSED** |
| `04-sweep.gb` | Channel 1 frequency sweep calculation | **PASSED** |
| `05-sweep details.gb` | Sweep shift, negation quirks, and auto-disable | **PASSED** |
| `06-overflow on trigger.gb` | Frequency overflow checks on re-trigger | **PASSED** |
| `07-len sweep period sync.gb` | Frame sequencer divider synchronization | **PASSED** |
| `08-len ctr during power.gb` | Length counter operation during APU power transitions | **PASSED** |
| `09-wave read while on.gb` | Cycle-accurate Wave RAM reading during active playback | **PASSED** |
| `10-wave trigger while on.gb` | DMG Wave RAM corruption quirk on channel re-trigger | **PASSED** |
| `11-regs after power.gb` | Audio register values after powering down APU | **PASSED** |
| `12-wave write while on.gb` | Wave RAM write redirection during active playback | **PASSED** |

### Additional Verified Test ROMs
- **Blargg `cpu_instrs`**: All 11 opcode validation tests pass.
- **Blargg `instr_timing`**: Verifies cycle durations of all SM83 instructions.
- **Blargg `mem_timing` & `mem_timing-2`**: Validates memory read/write cycle alignments.
- **Blargg `halt_bug`**: Validates HALT instruction pipeline behavior.

---

## Technology Stack

| Layer | Component | Technologies / Crates |
|---|---|---|
| **Core Architecture** | Emulation Engine | Rust (Edition 2024), `serde`, `serde-big-array`, `bincode` |
| **Desktop Client** | Windowing & Audio | SDL2 (`sdl2` with static link/bundled support) |
| | Rendering Engine | OpenGL 3.3 Core (`gl`) |
| | Graphical Interface | `egui`, `egui_sdl2_gl` |
| | File Dialogs | `rfd` |
| | Platform Utilities | `windows-sys` (`Win32_Media`) |
| **Web Client** | Wasm Runtime | `wasm-bindgen`, `web-sys`, `js-sys` |
| | Frontend Interface | Vanilla ES Modules, HTML5 Canvas, CSS Grid |
| | Web Audio Engine | Web Audio API, `AudioWorkletNode` |

---

## Repository Structure

```
GameBoy/
├── core/                       # Core emulation library (gb_core)
│   ├── src/
│   │   ├── apu/                # Audio Processing Unit & synthesis channels
│   │   │   ├── channel1.rs     # Square wave with sweep
│   │   │   ├── channel2.rs     # Square wave
│   │   │   ├── channel3.rs     # Custom wave RAM playback
│   │   │   ├── channel4.rs     # Noise channel (LFSR)
│   │   │   ├── length_counter.rs
│   │   │   ├── volume_envelope.rs
│   │   │   └── mod.rs          # APU coordinator & 512 Hz frame sequencer
│   │   ├── cart/               # Cartridge handling & MBC implementations
│   │   │   ├── rtc.rs          # MBC3 Real-Time Clock
│   │   │   └── mod.rs          # MBC1, MBC2, MBC3, MBC5 controller logic
│   │   ├── cpu/                # Sharp SM83 CPU
│   │   │   ├── opcodes.rs      # Complete instruction table & microcode
│   │   │   └── mod.rs          # Registers, interrupt servicing, halt bug
│   │   ├── ppu/                # Pixel Processing Unit
│   │   │   ├── modes.rs        # OAM scan, pixel transfer, H-Blank, V-Blank
│   │   │   ├── sprite.rs       # Sprite priority & extraction
│   │   │   ├── tile.rs         # Tile data decoding & palettes
│   │   │   └── mod.rs          # PPU coordinator & scanline engine
│   │   ├── bus.rs              # System memory bus routing
│   │   ├── io.rs               # Joypad input register
│   │   ├── timer.rs            # Divider and programmable timer
│   │   ├── wram.rs             # Working RAM & echo RAM
│   │   └── lib.rs              # Public library exports
│   └── tests/                  # Integration tests
│       └── dmg_sound_test.rs   # Blargg sound compliance runner
├── desktop/                    # Native desktop application
│   └── src/
│       ├── config.rs           # Persistent user preferences & keybinds
│       ├── debug.rs            # Interactive CLI debugger (gbd)
│       ├── gui.rs              # egui menu panels, palette picker, controls
│       └── main.rs             # Event pump, OpenGL pipeline, audio sync
├── wasm/                       # WebAssembly interface bindings
│   └── src/
│       └── lib.rs              # wasm-bindgen exports & JS bridge
├── html/                       # Web client distribution
│   ├── audio-processor.js      # AudioWorklet ring-buffer processor
│   ├── audio.js                # Web Audio API coordinator
│   ├── index.html              # Main web UI container
│   ├── index.js                # Canvas rendering & browser input handler
│   └── style.css               # Styling & responsive layout
└── test/                       # Test ROM binaries and test sources
    ├── cpu_instrs/             # Blargg CPU test suite
    ├── dmg_sound/              # Blargg APU test suite & singles
    └── instr_timing/           # Instruction timing suite
```

---

## Building and Running

### Prerequisites

- **Rust toolchain**: 1.85+ (Edition 2024 support).
- **C Compiler & CMake**: Required for SDL2 bundled build on native desktop.
- **wasm-pack**: Required if rebuilding the WebAssembly package.

### Desktop Application

Build and run in release mode for full hardware-accurate speed:

```bash
# Navigate to the desktop crate
cd desktop

# Run with a specific ROM
cargo run --release -- path/to/game.gb

# Alternatively, launch without arguments to open the file picker
cargo run --release
```

### WebAssembly / Web Application

1. Compile the `wasm` crate using `wasm-pack`:
   ```bash
   cd wasm
   wasm-pack build --target web --out-dir ../html/pkg
   ```

2. Serve the `html` folder through any local HTTP server (required for Web Workers and WASM loading):
   ```bash
   cd ../html
   # Using Python 3 built-in server:
   python -m http.server 8080
   ```

3. Open `http://localhost:8080` in any WebAudio/WASM-compatible browser.

### Running Test Suites

Execute the automated Blargg APU regression test suite:

```bash
cd core
cargo test --test dmg_sound_test -- --nocapture
```

---

## Controls

### Default Keyboard Controls

| Game Boy Input | Desktop Keybinding | Web Keybinding |
|---|---|---|
| **D-Pad Up** | Up Arrow | Up Arrow |
| **D-Pad Down** | Down Arrow | Down Arrow |
| **D-Pad Left** | Left Arrow | Left Arrow |
| **D-Pad Right** | Right Arrow | Right Arrow |
| **A Button** | `Z` | `Z` |
| **B Button** | `X` | `X` |
| **Select** | Right Shift | Right Shift / Left Shift |
| **Start** | Enter | Enter |

### Desktop Shortcuts

- **Ctrl + O**: Open ROM file dialog.
- **Space**: Hold for Fast-Forward.
- **P**: Pause / Resume execution.
- **F1 - F9**: Select Save State Slot 1–9.
- **F5**: Save state to active slot.
- **F7**: Load state from active slot.
- **F11**: Toggle Fullscreen.
- **Escape**: Exit application.

### Gamepad Controls
Plug-and-play controller detection via standard SDL2 mappings:
- **D-Pad**: Hat / Directional buttons
- **A / B**: Controller South / West buttons
- **Select / Start**: Back / Start buttons

---

## Debugger Reference

The desktop binary includes an integrated command-line debugger (`gbd`) accessible from the terminal running the application:

| Command | Arguments | Description |
|---|---|---|
| `b` | `<hex_addr>` | Sets an execution breakpoint at the specified PC address (e.g., `b 0x0100`). |
| `rb` | `<hex_addr>` | Sets a memory read watchpoint on the specified address. |
| `wb` | `<hex_addr>` | Sets a memory write watchpoint on the specified address. |
| `r` | - | Dumps all CPU registers (`AF`, `BC`, `DE`, `HL`, `SP`, `PC`) and active flags (`Z`, `N`, `H`, `C`). |
| `m` / `dump` | `<hex_addr> [len]` | Dumps memory contents starting at `<hex_addr>` for `len` bytes. |
| `s` | - | Steps forward by a single instruction. |
| `c` | - | Resumes normal continuous execution. |

---

## License & Acknowledgments

- **Pan Docs**: Indispensable hardware documentation provided by the Game Boy development community.
- **Blargg**: Test ROM suites (`cpu_instrs`, `dmg_sound`, `mem_timing`, `halt_bug`).
- **SameBoy & Gambatte**: Reference implementations for subtle hardware edge cases.

