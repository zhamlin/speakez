import * as libopus from "/js/lib/libopus.js";
import { ResumableReader, RingBuffer, AudioWriter } from "/js/lib/ringbuf.js";

const state = {
	channelCount: 0,
	interval: null,
};

/** @type {libopus.Decoder} */
let decoder;

/** @type {ResumableReader} */
let reader;

/** @type {AudioWriter} */
let writer;

async function init(input) {
	try {
		await libopus.init();
	} catch (err) {
		console.log("error loading libopus", err);
	}

	state.channelCount = input.channelCount;

	decoder = new libopus.Decoder({
		channels: state.channelCount,
	});

	const rrb = new RingBuffer(input.readerSab, Uint8Array);
	const staging_buffer = new Uint8Array(4096);
	reader = new ResumableReader(rrb, staging_buffer, "tag");

	const wrb = new RingBuffer(input.writerSab, Float32Array);
	writer = new AudioWriter(wrb);

	state.interval = setInterval(readFromQueue, 50);
}

function readFromQueue() {
	let msg = reader.readSizedMessage();
	while (msg) {
		decode_and_play(msg);
		msg = reader.readSizedMessage();
	}
}

function decode_and_play(encodedData) {
	const pcm = decoder.decode(encodedData);
	const size = writer.enqueue(pcm);

	if (size !== pcm.length) {
		const msgs = [
			`processor worklet not running dequeue fast enough: size=${size} pcm.length=${pcm.length}`,
			"check that the sample rate is correct",
		];
		console.error(msgs.join("\n"));
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
