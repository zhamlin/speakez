import * as libopus from "/js/lib/libopus.js";
import { writeSizedMessage, RingBuffer, AudioReader } from "/js/lib/ringbuf.js";

const state = {
	channelCount: 0,
	sampleRate: 0,
	frameSize: 0,
	interval: null,
};

/** @type {Float32Array} */
let staging_buffer;

/** @type {libopus.Encoder} */
let encoder;

/** @type {AudioReader} */
let reader;

/** @type {RingBuffer} */
let writer;

async function init(input) {
	try {
		await libopus.init();
	} catch (err) {
		console.log("error loading libopus", err);
	}

	state.channelCount = input.channelCount;
	state.frameSize = input.frameSize;
	state.sampleRate = input.sampleRate;

	encoder = new libopus.Encoder({
		application: libopus.Application.AUDIO,
		channels: state.channelCount,
	});

	const rrb = new RingBuffer(input.readerSab, Float32Array);
	reader = new AudioReader(rrb);
	writer = new RingBuffer(input.writerSab, Uint8Array);

	// A smaller staging_buffer array to copy the audio samples from, before conversion
	// to uint16. It's size is 4 times less than the 1 second worth of data
	// that the ring buffer can hold, so it's 250ms, allowing to not make
	// deadlines:
	// staging buffer size = ring buffer size / sizeof(float32) / stereo / 4
	// staging_buffer = new Float32Array(input.readerSab.byteLength / 4 / 4 / 2);
	staging_buffer = new Float32Array(state.frameSize);
	state.interval = setInterval(readFromMicQueue, 50);
}

function readFromMicQueue() {
	while (reader.availableRead() >= state.frameSize) {
		reader.dequeue(staging_buffer);

		const encodedData = encoder.encode(staging_buffer);
		if (encodedData.length <= 3) {
			// DTX likely enabled
			continue;
		}

		if (!writeSizedMessage(encodedData, writer)) {
			console.log("recorder can not write data, mumble may be behind");
		}
	}
}

self.onmessage = async (e) => {
	switch (e.data.command) {
		case "init": {
			await init(e.data);
			break;
		}
	}
};

postMessage("init");
