import createOpusModule from "/js/native/opus/libopus.js";

/**
 * @param {object} [opts={}] - Options for the encoder
 * @param {(8000|12000|16000|24000|48000)} [opts.rate=48000] - Sampling rate of input signal (Hz)
 * @param {number} [opts.channels=1] - Number of (interleaved) channels
 * @param {Application} [opts.application=AUDIO] - Encoding mode
 */

function opusErrCodeToStr(err) {
	const errorStringPtr = libopus._opus_strerror(err);
	if (errorStringPtr > 0) {
		// convert the pointer to a string
		return libopus.UTF8ToString(errorStringPtr);
	}
	return `unhandled error code: ${err}`;
}

const OPUS_SET_BITRATE = 4002; // from opus_defines.h
const OPUS_GET_CHANNELS = 4029;
const OPUS_AUTO = -1000; // from opus_defines.h
const OPUS_SET_DTX = 4016; // Control request code to set DTX

/**
 * Enum for application types.
 * @readonly
 * @enum {number}
 */
export const Application = {
	VOIP: 2048,
	AUDIO: 2049,
	RESTRICTED_LOWDELAY: 2051,
};

const isValidApplication = (app) => {
	return (
		app === Application.VOIP ||
		app === Application.AUDIO ||
		app === Application.RESTRICTED_LOWDELAY
	);
};

const validSampleRates = [8000, 12000, 16000, 24000, 48000];
const isValidSampleRate = (rate) => {
	return validSampleRates.includes(rate);
};

const validChannels = [1, 2];
const isValidChannelCount = (count) => {
	return validChannels.includes(count);
};

// Note that the opus documentation is not consistent with that 120ms
// that is suggested in the description of opus_decode. In other places
// such as the overview of the Opus Encoder, 60ms is used as the upper
// limit.
// To be on the safe side, 120ms has been choosen here.
const pcm_len = 4 /*Float32*/ * 2 /*channels*/ * 120 /*ms*/ * 48 /*samples/ms*/;
// const pcm_len = 4096 * 4;
const maxOutputBytes = 120 /*ms*/ * 512 /*bits per ms*/;

let libopus;
export async function init() {
	libopus = await createOpusModule();
}

/**
 * @param {number} sampelRate
 * @param {1|2} channels
 * @param {Application} application
 */
const newOpusEncoder = (sampelRate, channels, application, { enableDTX }) => {
	if (!isValidSampleRate(sampelRate)) {
		throw `invalid sample rate: ${sampelRate}`;
	}

	if (!isValidApplication(application)) {
		throw `invalid application: ${application}`;
	}

	if (!isValidChannelCount(channels)) {
		throw `invalid channel count: ${channels}`;
	}

	const size = libopus._opus_encoder_get_size(channels);
	const memory = libopus._malloc(size);
	// Initialize the encoder
	const ret = libopus._opus_encoder_init(
		memory,
		sampelRate,
		channels,
		application,
	);
	if (ret !== 0) {
		libopus._free(memory);
		throw `_opus_encoder_init: ${opusErrCodeToStr(ret)}`;
	}

	if (enableDTX) {
		const result = libopus._opus_encoder_ctl(memory, OPUS_SET_DTX, 1); // Enable DTX (1)
		if (result !== 0) {
			libopus._free(memory);
			throw new Error(`_opus_encoder_ctl: ${opusErrCodeToStr(result)}`);
		}
	}

	// TODO: opus_encoder_ctl(encoder, OPUS_SET_BITRATE(BITRATE));
	// https://github.com/Johni0702/mumble-client-codecs-browser/blob/6a8c676bcf3a37fe94371f46b3f6a810178df1de/src/encode-worker.js#L31
	// store bitrate in varargs array
	// libopus.HEAP32[val >> 2] = bitrate || OPUS_AUTO;
	// const ret1 = libopus._opus_encoder_ctl(enc, OPUS_SET_BITRATE, val);
	// if (ret1 !== 0) {
	// 	throw new Error(libopus.Pointer_stringify(libopus._opus_strerror(ret1)));
	// }

	return memory;
};

export class Encoder {
	/**
	 * @param {Object} opts
	 * @param {1|2} opts.channels
	 * @param {Application} opts.application
	 */
	constructor(opts) {
		this._sampleRate = 48000;
		this._channelCount = opts.channels;
		this._application = opts.application;

		this._encoder = newOpusEncoder(
			this._sampleRate,
			this._channelCount,
			this._application,
			{ enableDTX: true },
		);

		this._pcmPtr = libopus._malloc(pcm_len);
		this._outputPtr = libopus._malloc(maxOutputBytes);
	}

	cleanup() {
		libopus._free(this._encoder);
		libopus._free(this._pcmPtr);
		libopus._free(this._outputPtr);
	}

	/**
	 * Encodes an array of (interleaved) pcm samples.
	 * One frame must be exactly 2.5, 5, 10, 20, 40 or 60ms.
	 *
	 * @param {Float32Array} pcm - Input samples
	 * @returns {Uint8Array} The encoded output
	 */
	encode(pcm) {
		const frameSize = pcm.length / this._channelCount;

		if (pcm.length * 4 > pcm_len) {
			throw new Error("pcm array too large");
		}

		// Copy the float array into WebAssembly memory
		libopus.HEAPF32.set(pcm, this._pcmPtr / 4);
		const len = libopus._opus_encode_float(
			this._encoder,
			this._pcmPtr,
			frameSize,
			this._outputPtr,
			maxOutputBytes,
		);
		if (len < 0) {
			throw `_opus_encode_float: ${opusErrCodeToStr(len)}`;
		}

		const encodedData = libopus.HEAPU8.subarray(
			this._outputPtr,
			this._outputPtr + len,
		);

		return encodedData;
	}
}

/**
 * @param {number} sampleRate
 * @param {1|2} channels
 */
function newOpusDecoder(sampleRate, channels) {
	const errorPtr = libopus._malloc(4);
	const decoderPtr = libopus._opus_decoder_create(
		sampleRate,
		channels,
		errorPtr,
	);

	const errorCode = libopus.getValue(errorPtr, "i32");
	if (errorCode !== 0) {
		libopus._free(errorPtr);
		throw `Failed to create Opus decoder: Error code ${errorCode}`;
	}

	libopus._free(errorPtr);
	return decoderPtr;
}

// Misc:OPUS FEC: https://ddanilov.me/how-to-enable-in-band-fec-for-opus-codec/

export class Decoder {
	/**
	 * @param {Object} opts
	 * @param {1|2} opts.channels
	 */
	constructor(opts) {
		this._sampleRate = 48000;
		this._channelCount = opts.channels;

		this._decoder = newOpusDecoder(this._sampleRate, this._channelCount);
		// used to hold decoded pcm data
		this._pcmPtr = libopus._malloc(pcm_len);
		// used to hold encoded pcm data
		this._encodedPtr = libopus._malloc(pcm_len);
	}

	cleanup() {
		libopus._free(this._decoder);
		libopus._free(this._pcmPtr);
		libopus._free(this._encodedPtr);
	}

	/**
	 * @param {Uint8Array} data - encoded opus packet
	 * @returns {number} Number of channels in the packet
	 */
	getNumberOfChannels(data) {
		libopus.HEAPU8.set(data, this._encodedPtr);
		const channels = libopus._opus_packet_get_nb_channels(this._encodedPtr);
		if (channels <= 0) {
			throw `_opus_packet_get_nb_channels: ${opusErrCodeToStr(samples)}`;
		}
		return channels;
	}

	/**
	 * @param {Uint8Array} data - encoded opus packet
	 * @returns {number} Number of samples in the packet
	 */
	getNumberOfSamples(data) {
		libopus.HEAPU8.set(data, this._encodedPtr);
		const samples = libopus._opus_packet_get_nb_samples(
			this._encodedPtr,
			data.length,
			this._sampleRate,
		);
		if (samples <= 0) {
			throw `_opus_packet_get_nb_samples: ${opusErrCodeToStr(samples)}`;
		}
		return samples;
	}

	/**
	 * Decode an array of (interleaved) pcm samples.
	 *
	 * @param {Uint8Array} encodedData - Input data
	 * @returns {Float32Array} The pcm data
	 */
	decode(encodedData) {
		const channelCount = this.getNumberOfChannels(encodedData);

		const frameSize = libopus._opus_decode_float(
			this._decoder,
			this._encodedPtr,
			encodedData.length,
			this._pcmPtr,
			maxOutputBytes,
			0, // 0 = no FEC
		);

		if (frameSize < 0) {
			console.error("Decoding failed:", opusErrCodeToStr(frameSize));
			return;
		}

		// TODO: libopus._opus_decoder_ctl(this._decoder, CHANNELCOUNT)

		const decodedPcm = new Float32Array(
			libopus.HEAPF32.buffer,
			this._pcmPtr,
			frameSize * channelCount,
		);
		return decodedPcm;
	}
}
