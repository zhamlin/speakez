export const MUMBLE_SAMPLE_RATE = 48000;

// Function to play the decoded PCM data using Web Audio API
export function playPcmData(audioContext, pcmData, sampleRate, channels) {
	// Create an AudioBuffer with the decoded PCM data
	const frameCount = pcmData.length / channels;
	// console.log(frameCount);
	const buffer = audioContext.createBuffer(
		channels, // Number of channels
		frameCount, // Number of frames (samples per channel)
		sampleRate, // Sample rate
	);

	// Split PCM data into channels
	for (let channel = 0; channel < channels; channel++) {
		const channelData = buffer.getChannelData(channel);
		for (let i = 0; i < channelData.length; i++) {
			channelData[i] = pcmData[i * channels + channel]; // Interleaved PCM data
		}
	}

	// Create a buffer source and play the decoded audio
	const source = audioContext.createBufferSource();
	source.buffer = buffer;
	source.connect(audioContext.destination);
	source.start(0);
}

export function calculateFrameSize(sampleRate, frameDuration) {
	return (sampleRate / 1000) * frameDuration;
}

export function setupPlayback(ctx, sab) {
	// const sab2 = RingBuffer.getStorageForCapacity(31, Uint8Array);
	// const rb2 = new RingBuffer(sab2, Uint8Array);
	// paramWriter = new ParameterWriter(rb2);

	const n = new AudioWorkletNode(ctx, "processor", {
		processorOptions: {
			audioQueue: sab,
			// paramQueue: sab2,
		},
	});
	n.connect(ctx.destination);
	return n;
}

/**
 * @param {MediaStreamAudioSourceNode} source
 */
function stereoSourceToMono(ctx, source) {
	DEBUG && console.log("down mixing stero input to mono");

	// Split the stereo channels
	const splitter = ctx.createChannelSplitter(2);

	// Sum the two channels to mono
	const merger = ctx.createChannelMerger(1);

	source.connect(splitter);

	// Connect both channels to the first input of the merger (summing them)
	splitter.connect(merger, 0, 0); // Left channel
	splitter.connect(merger, 1, 0); // Right channel

	return merger;
}

/**
 * @param {AudioContext} ctx
 * @param {MediaStream} stream
 */
export function newMonoSource(ctx, stream) {
	let source = ctx.createMediaStreamSource(stream);
	if (source.channelCount === 2) {
		source = stereoSourceToMono(ctx, source);
	} else if (source.channelCount !== 1) {
		throw `unexpected channel count: ${source.channelCount}`;
	}

	return source;
}

export async function createContext() {
	const audioCtx = new AudioContext({
		sampleRate: MUMBLE_SAMPLE_RATE,
		latencyHint: "interactive",
	});
	await audioCtx.audioWorklet.addModule(
		"/js/audio/processors/shared-playback.js",
	);
	await audioCtx.audioWorklet.addModule(
		"/js/audio/processors/record-processor.js",
	);
	return audioCtx;
}

/**
 * @param {SharedArrayBuffer} readerSab
 * @param {SharedArrayBuffer} writerSab
 */
export async function setupAudioRecorderWorker(readerSab, writerSab) {
	// mumble only supports 10ms, with the option
	// to group N frames, vs increaing the frameSize
	// https://github.com/mumble-voip/mumble/issues/6415
	// https://github.com/mumble-voip/mumble/blob/dfc8dacae3f37ecd522d73fe7db9a0a91060e0ae/src/mumble/AudioInput.h#L219-L224
	// https://github.com/mumble-voip/mumble/blob/dfc8dacae3f37ecd522d73fe7db9a0a91060e0ae/src/mumble/AudioInput.h#L235-L239
	const frameDurationMS = 10;
	const frameSize = calculateFrameSize(MUMBLE_SAMPLE_RATE, frameDurationMS);
	DEBUG && console.log("recorder frame size: ", frameSize);

	const worker = new Worker(new URL("workers/recorder.js", import.meta.url), {
		type: "module",
	});

	worker.onerror = (event) => {
		console.log("main thread recieved error from audio recorder worker", event);
	};

	await new Promise((resolve) => {
		worker.onmessage = (event) => {
			if (event.data === "init") {
				resolve();
			}
		};
	});

	worker.onmessage = (e) => {
		console.log("main thread recieved event from audio recorder worker", e);
	};

	worker.postMessage({
		command: "init",
		sampleRate: MUMBLE_SAMPLE_RATE,
		channelCount: 1,
		frameSize,
		writerSab,
		readerSab,
	});

	return worker;
}

/**
 * @param {SharedArrayBuffer} readerSab
 * @param {SharedArrayBuffer} writerSab
 */
export async function setupAudioPlaybackWorker(readerSab, writerSab) {
	const worker = new Worker(new URL("workers/playback.js", import.meta.url), {
		type: "module",
	});

	worker.onerror = (event) => {
		console.log("main thread recieved error from audio playback worker", event);
	};

	await new Promise((resolve) => {
		worker.onmessage = (event) => {
			if (event.data === "init") {
				resolve();
			}
		};
	});

	worker.onmessage = (e) => {
		console.log("main thread recieved event from audio playback worker", e);
	};

	worker.postMessage({
		command: "init",
		channelCount: 1,
		writerSab,
		readerSab,
	});

	return worker;
}
