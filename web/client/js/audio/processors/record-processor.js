import {
	AudioWriter,
	RingBuffer,
	interleave,
} from "/js/third_party/ringbuf/index.js";

class RecorderWorklet extends AudioWorkletProcessor {
	constructor(options) {
		super();
		this.channelCount = options.processorOptions.channelCount;
		// Staging buffer to interleave the audio data.
		this.bufSize = 128 * this.channelCount; // 1=mono,2=stereo
		this.interleaved = new Float32Array(this.bufSize);

		const sab = options.processorOptions.buffer;
		this._audio_writer = new AudioWriter(new RingBuffer(sab, Float32Array));
	}

	process(inputs, outputs, _parameters) {
		const input = inputs[0];
		const output = outputs[0];

		if (input && input.length > 0) {
			// interleave and store in the queue
			interleave(input, this.interleaved);
			const enqueued = this._audio_writer.enqueue(this.interleaved);

			if (enqueued !== 128) {
				console.log(
					"underrun: the recorder worker doesn't dequeue fast enough!",
				);
			}

			// forward data to other nodes
			for (let channel = 0; channel < input.length; channel++) {
				const inputChannel = input[channel];
				const outputChannel = output[channel];

				// Copy data from input to output channel
				for (let i = 0; i < inputChannel.length; i++) {
					outputChannel[i] = inputChannel[i];
				}
			}
		}
		return true;
	}
}

registerProcessor("recorder-worklet", RecorderWorklet);
