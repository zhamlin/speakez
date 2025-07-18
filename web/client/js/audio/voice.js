import { clamp } from "/js/lib/util.js";
import { MUMBLE_SAMPLE_RATE, newMonoSource } from "./audio.js";

export async function getLocalStream() {
	try {
		const stream = await navigator.mediaDevices.getUserMedia({
			audio: {
				sampleRate: MUMBLE_SAMPLE_RATE,
				echoCancellation: false,
				autoGainControl: false,
				noiseSuppression: false,
				latency: 0,
			},
		});
		return stream;
	} catch (err) {
		alert(`getLocalStream: ${err}`);
	}
}

/**
 * @param {AudioContext} ctx
 */
function newAnalyzerNode(ctx) {
	const analyser = ctx.createAnalyser();
	analyser.smoothingTimeConstant = 0.8;
	analyser.fftSize = 1024;
	const array = new Uint8Array(analyser.frequencyBinCount);

	return {
		node: analyser,
		array: array,
	};
}

/**
 * @param {AudioContext} ctx
 */
function newGainNode(ctx) {
	return {
		node: ctx.createGain(),
		setDecibels: function (dB) {
			this.setValue(10 ** (dB / 20));
		},
		swapValue: function (value) {
			const last = this.node.gain.value;
			this.setValue(value);
			return last;
		},
		setValue: function (n) {
			const value = clamp(n, 0, 20);
			this.node.gain.value = value;
		},
	};
}

/**
 * @param {AudioContext} ctx
 * @param {SharedArrayBuffer} buffer
 */
function newRecorderNode(ctx, buffer) {
	const typ = "recorder-worklet";
	const recorderNode = new AudioWorkletNode(ctx, typ, {
		processorOptions: {
			buffer,
			channelCount: 1,
		},
	});

	return recorderNode;
}

/**
 * @param {AudioContext} ctx
 * @param {number} desiredRate
 */
function checkSampleSize(ctx, desiredRate) {
	console.assert(
		ctx.sampleRate === desiredRate,
		"sample rate not applied",
		ctx.sampleRate,
	);
}

/**
 * @param {AnalyserNode} analyser
 * @param {Uint8Array} array
 */
function averageVolumeLevel(analyser, array) {
	analyser.getByteFrequencyData(array);
	const arraySum = array.reduce((a, value) => a + value, 0);
	const average = arraySum / array.length;
	return Math.round(average);
}

export class Mic {
	state = {
		monitoring: false,
		muted: false,
		lastGainValue: 0,
	};

	/**
	 * @param {Object} opts
	 * @param {function(Float32Array):void} opts.onvoicedata
	 * @constructor
	 */
	constructor({ buffer }) {
		this.started = false;
		this.buffer = buffer;
	}

	/**
	 * @param {AudioContext} ctx
	 * @param {MediaStream} stream
	 */
	setSource(ctx, stream) {
		checkSampleSize(ctx, MUMBLE_SAMPLE_RATE);

		if (this.state.started) {
			this.disconnect();
		}

		this.audioCtx = ctx;
		this.stream = stream;
		this.source = newMonoSource(this.audioCtx, this.stream);

		this.gain = newGainNode(this.audioCtx);
		this.analyser = newAnalyzerNode(this.audioCtx);

		this.recorder = newRecorderNode(this.audioCtx, this.buffer);
	}

	calculateVolumeLevel() {
		if (!this.analyser) {
			return 0;
		}

		return averageVolumeLevel(this.analyser.node, this.analyser.array);
	}

	setDecibelAdjustment(dB) {
		this.gain.setDecibels(dB);
	}

	monitor(enabled) {
		if (this.state.monitoring === enabled) {
			return;
		}

		if (this.started) {
			this.stop();
			this.state.monitoring = enabled;
			this.start();
		} else {
			this.state.monitoring = enabled;
		}
	}

	start() {
		if (this.started) {
			return;
		}

		// handle state being loaded
		if (this.state.muted) {
			this.mute(true, true);
		}

		const node = this.source
			.connect(this.gain.node)
			.connect(this.analyser.node)
			.connect(this.recorder);

		if (this.state.monitoring) {
			node.connect(this.audioCtx.destination);
		}

		this.started = true;
	}

	stop() {
		if (!this.started) {
			return;
		}

		if (!this.source) {
			return;
		}

		this.source.disconnect();
		this.gain.node.disconnect();
		this.analyser.node.disconnect();
		this.recorder.disconnect();

		if (this.state.monitoring) {
			this.audioCtx.destination.disconnect();
		}

		this.started = false;
	}

	disconnect() {
		this.stop();

		if (!this.stream) {
			return;
		}

		// Stop all audio tracks to release the microphone
		const tracks = this.stream.getTracks();
		for (const track of tracks) {
			track.stop();
		}
	}

	mute(wantMute, force) {
		this.state.muted = wantMute;

		const hasGainNode = this.gain !== undefined;
		if (!hasGainNode) {
			return;
		}

		let gainValue = this.state.lastGainValue;
		if (this.state.muted && (force || gainValue !== 0)) {
			gainValue = 0;
		}
		this.state.lastGainValue = this.gain.swapValue(gainValue);
	}
}
