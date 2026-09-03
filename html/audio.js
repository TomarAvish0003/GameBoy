export class AudioManager {
    constructor() {
        this.ctx = null;
        this.workletNode = null;
        this.gainNode = null;
        this.isInitialized = false;
        this.volume = 1.0;
    }

    async init() {
        if (this.isInitialized) return;

        const AudioContextClass = window.AudioContext || window.webkitAudioContext;
        this.ctx = new AudioContextClass({
            sampleRate: 44100,
            latencyHint: 'interactive'
        });

        // Load the audio worklet module
        await this.ctx.audioWorklet.addModule('audio-processor.js');

        this.workletNode = new AudioWorkletNode(this.ctx, 'gameboy-audio-processor', {
            numberOfInputs: 0,
            numberOfOutputs: 1,
            outputChannelCount: [2]
        });

        this.gainNode = this.ctx.createGain();
        this.gainNode.gain.setValueAtTime(this.volume, this.ctx.currentTime);

        this.workletNode.connect(this.gainNode);
        this.gainNode.connect(this.ctx.destination);

        this.isInitialized = true;
        this.attachAutoplayUnlock();
    }

    attachAutoplayUnlock() {
        const unlock = () => {
            if (this.ctx && this.ctx.state === 'suspended') {
                this.ctx.resume();
            }
            window.removeEventListener('click', unlock);
            window.removeEventListener('keydown', unlock);
            window.removeEventListener('touchstart', unlock);
        };

        window.addEventListener('click', unlock);
        window.addEventListener('keydown', unlock);
        window.addEventListener('touchstart', unlock);
    }

    pushSamples(samples) {
        if (!this.isInitialized || !this.workletNode || samples.length === 0) {
            return;
        }

        // Float32Array from wasm gets posted directly to the AudioWorklet
        this.workletNode.port.postMessage({
            type: 'samples',
            samples: samples
        });
    }

    clear() {
        if (this.workletNode) {
            this.workletNode.port.postMessage({ type: 'clear' });
        }
    }

    setVolume(vol) {
        this.volume = Math.max(0.0, Math.min(1.0, vol));
        if (this.gainNode && this.ctx) {
            this.gainNode.gain.setValueAtTime(this.volume, this.ctx.currentTime);
        }
    }

    resume() {
        if (this.ctx && this.ctx.state === 'suspended') {
            this.ctx.resume();
        }
    }
}