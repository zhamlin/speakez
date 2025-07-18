import { html, render } from "html";

import { App } from "./views/app.js";
import {
	createStateWithViews as createAppState,
	newDispatch,
	newMumbleReducer,
} from "./state.js";

import { Mic, getLocalStream } from "./audio/voice.js";
import {
	createContext,
	setupAudioRecorderWorker,
	setupAudioPlaybackWorker,
	setupPlayback,
	MUMBLE_SAMPLE_RATE,
} from "./audio/audio.js";
import { RingBuffer } from "/js/lib/ringbuf.js";

import { setupMumbleWorker, MumbleEvent, postRequest } from "./mumble/index.js";
import { Client } from "./mumble/client.js";
import { effect } from "@preact/signals";

window.DEBUG = true;

function featureDetection() {
	return {
		hasAudioWorklet: typeof AudioWorkletNode !== "undefined",
		hasWorker: typeof Worker !== "undefined",
		hasWebsocket: typeof WebSocket !== "undefined",
		hasWebassembly: typeof WebAssembly !== "undefined",
		hasWebTransport: typeof WebTransport !== "undefined",
		hasAudioEncoder: typeof AudioEncoder !== "undefined",
		hasAudioDecoder: typeof AudioDecoder !== "undefined",
		hasSharedArrayBuffer: typeof SharedArrayBuffer !== "undefined",
		hasTauri: typeof __TAURI__ !== "undefined",
	};
}

/**
 * @param {ReturnType<typeof featureDetection>} features
 */
function confirmFeatureSupport(features) {
	if (!features.hasAudioWorklet) {
		alert("AudioWorklet support is required");
	}
	if (!features.hasWorker) {
		alert("Web Worker support is required");
	}
	if (!features.hasWebsocket && !features.hasWebTransport) {
		alert("Websocket support is required");
	}
	if (!features.hasWebassembly) {
		alert("WebAssembly support is required");
	}
}

const features = featureDetection();
DEBUG && console.log(features);
confirmFeatureSupport(features);

// const state = createState(createAppState());
const state = createAppState();
window.state = state;

const bufferSeconds = 0.2;
const bufferSamples = MUMBLE_SAMPLE_RATE * bufferSeconds;

// Used by the recording audio worklet as a writer, and used as a reader
// by the recording worker.
const recordingBuf = RingBuffer.getStorageForCapacity(
	bufferSamples,
	Float32Array,
);

// Used by the playback audio worklet as a reader, and as a writer by the
// playback worker.
const playbackBuf = RingBuffer.getStorageForCapacity(
	bufferSamples,
	Float32Array,
);

// Used as a writer by the recording worker, and as reader by the mumble worker.
const mumbleOutBuf = RingBuffer.getStorageForCapacity(4096 * 3, Uint8Array);

// Used as a reader by the playback worker, and as writer by the mumble worker.
const mumbleInBuf = RingBuffer.getStorageForCapacity(4096 * 3, Uint8Array);

const worker = await setupMumbleWorker(mumbleOutBuf, mumbleInBuf);
const client = new Client((msg, data) => {
	return postRequest(
		{
			command: msg,
			...data,
		},
		worker,
	);
});

const mumbleReducer = newMumbleReducer(state);

/**
 * @param {MessageEvent<MumbleEvent>} e
 */
worker.onmessage = (e) => {
	if (e.data._tag) {
		return;
	}

	mumbleReducer(e.data);
};
window.mumble = client;

/**
 * @param {RingBuffer} recordingBuf
 * @param {import("./state.js").State} state
 */
const setupMic = (recordingBuf, state) => {
	const mic = new Mic({ buffer: recordingBuf });
	mic.monitor(state.mic.monitoring.value);
	mic.mute(state.mic.muted.value);

	effect(() => mic.monitor(state.mic.monitoring.value));
	effect(() => mic.mute(state.mic.muted.value));

	return mic;
};
const mic = setupMic(recordingBuf, state);
window.mic = mic;

const dispatch = newDispatch(state, { mumbleWorker: client, mic });
window.dispatch = dispatch;

const body = document.querySelector("body");
render(html`<${App} state=${state} dispatch=${dispatch} />`, body);

/**
 * @returns {Promise<[AudioContext]>}
 */
async function setupAudio() {
	const ctx = await createContext();
	setupPlayback(ctx, playbackBuf);

	await Promise.all([
		setupAudioRecorderWorker(recordingBuf, mumbleOutBuf),
		setupAudioPlaybackWorker(mumbleInBuf, playbackBuf),
		getLocalStream().then((stream) => mic.setSource(ctx, stream)),
	]);
	return ctx;
}

const _audioCtx = await setupAudio();

mic.gain.setValue(1);
mic.start();

navigator.mediaDevices.addEventListener("devicechange", () => {
	console.log("devicechange");
});
