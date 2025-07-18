import { AudioReader, RingBuffer } from "/js/third_party/ringbuf/index.js";

class Processor extends AudioWorkletProcessor {
	static get parameterDescriptors() {
		return [];
	}

	constructor(options) {
		super(options);

		this.bufSize = 128;
		this.interleaved = new Float32Array(this.bufSize);
		this.amp = 1.0;
		this.o = { index: 0, value: 0 };
		const { audioQueue, paramQueue } = options.processorOptions;
		this._audio_reader = new AudioReader(
			new RingBuffer(audioQueue, Float32Array),
		);
		// this._param_reader = new ParameterReader(
		//   new RingBuffer(paramQueue, Uint8Array)
		// );
	}

	process(inputs, outputs, parameters) {
		// Get any param changes
		// if (this._param_reader.dequeue_change(this.o)) {
		//   this.amp = this.o.value;
		// }

		if (this._audio_reader.available_read() >= this.bufSize) {
			// read 128 frames from the queue, deinterleave, and write to output
			// buffers.
			this._audio_reader.dequeue(this.interleaved);

			const channel_count = this.interleaved.length / 128;
			for (let i = 0; i < this.interleaved.length; i++) {
				outputs[0][0][i] = this.amp * this.interleaved[i];
			}
		}

		return true;
	}
}

registerProcessor("processor", Processor);
