class GameBoyAudioProcessor extends AudioWorkletProcessor {
    constructor() {
        super();
        // 16384 samples (~370ms maximum cushion, typical fill is ~20ms)
        this.BUFFER_SIZE = 16384;
        this.bufferL = new Float32Array(this.BUFFER_SIZE);
        this.bufferR = new Float32Array(this.BUFFER_SIZE);
        this.writePtr = 0;
        this.readPtr = 0;

        this.port.onmessage = (event) => {
            const data = event.data;
            if (data.type === 'samples') {
                this.pushSamples(data.samples);
            } else if (data.type === 'clear') {
                this.readPtr = 0;
                this.writePtr = 0;
            }
        };
    }

    pushSamples(interleaved) {
        const frames = interleaved.length / 2;
        let availableSpace = this.BUFFER_SIZE - this.getBufferedCount();

        // Prevent backlog/latency accumulation (clamp latency under ~40ms)
        const MAX_LATENCY_SAMPLES = 2048;
        if (this.getBufferedCount() > MAX_LATENCY_SAMPLES) {
            // Drop older samples to keep latency pinned to real-time
            this.readPtr = (this.writePtr - 512 + this.BUFFER_SIZE) % this.BUFFER_SIZE;
        }

        for (let i = 0; i < frames; i++) {
            this.bufferL[this.writePtr] = interleaved[i * 2];
            this.bufferR[this.writePtr] = interleaved[i * 2 + 1];
            this.writePtr = (this.writePtr + 1) % this.BUFFER_SIZE;
        }
    }

    getBufferedCount() {
        if (this.writePtr >= this.readPtr) {
            return this.writePtr - this.readPtr;
        }
        return this.BUFFER_SIZE - (this.readPtr - this.writePtr);
    }

    process(inputs, outputs, parameters) {
        const output = outputs[0];
        const outL = output[0];
        const outR = output[1];
        const count = outL.length; // Always 128 samples

        for (let i = 0; i < count; i++) {
            if (this.readPtr !== this.writePtr) {
                outL[i] = this.bufferL[this.readPtr];
                outR[i] = this.bufferR[this.readPtr];
                this.readPtr = (this.readPtr + 1) % this.BUFFER_SIZE;
            } else {
                // Buffer underrun (silence)
                outL[i] = 0.0;
                outR[i] = 0.0;
            }
        }

        return true;
    }
}

registerProcessor('gameboy-audio-processor', GameBoyAudioProcessor);