import { Client } from "./client.js";

/**
 * Waits for the first message that isMatch returns true for.
 * @param {(msg: any) => boolean} isMatch
 * @param {Worker} worker
 */
function waitForMessage(isMatch, worker) {
	return new Promise((resolve, reject) => {
		const finish = (fn) => {
			worker.removeEventListener("message", handleMessage);
			worker.removeEventListener("error", handleError);
			fn();
		};

		const handleError = (event) => {
			finish(() => reject(new Error(`Worker error: ${event.message}`)));
		};

		const handleMessage = (event) => {
			if (isMatch(event)) {
				finish(() => resolve(event));
			}
		};

		worker.addEventListener("error", handleError);
		worker.addEventListener("message", handleMessage);
	});
}

/**
 * @param {Worker} worker
 */
function waitForAnyMessage(worker) {
	return waitForMessage(() => true, worker);
}

/**
 * Send a message to the worker and return a promise that
 * resolves when a response with the same tag is received.
 *
 * @template Req Request
 * @template Resp Response
 * @param {Req} message
 * @param {Worker} worker
 * @returns {Promise<Resp>}
 */
export async function postRequest(message, worker) {
	const id = Math.random().toString(36);
	const isTaggedMsg = ({ data: { _tag } }) => _tag === id;
	const msg = waitForMessage(isTaggedMsg, worker).then(
		(result) => result.data.data,
	);

	worker.postMessage({ _tag: id, ...message });
	return await msg;
}

export class MumbleWorker {
	/** @param {Worker} worker */
	constructor(worker) {
		this._worker = worker;
	}

	_postRequest(msg) {
		return postRequest(msg, this._worker);
	}

	_sendMessage(msg) {
		this._worker.postMessage(msg);
	}

	/**
	 * @typedef {Object} Connected
	 * @property session {number}
	 * @property users {Array<User>}
	 * @property channels {Array<Channel>}
	 * */

	/**
	 * @returns {Promise<Connected>}
	 * returns a promise that resolves when the
	 * handshake has completed
	 */
	connect({ addr, auth }) {
		return this._postRequest({
			command: "connect",
			addr,
			auth,
		});
	}

	disconnect() {
		this._sendMessage({
			command: "disconnect",
		});
	}

	/**
	 * @returns {Promise<any>}
	 */
	switchChannel(channelID) {
		return this._postRequest({
			command: "switch_channel",
			channelID,
		});
	}

	/**
	 * @param {number} id
	 * @returns {Promise<User>}
	 */
	getUser(id) {
		return this._postRequest({
			query: "user",
			id,
		});
	}

	/** @returns {Promise<Array<User>>} */
	getUsers() {
		return this._postRequest({
			query: "users",
		});
	}

	/** @returns {Promise<Array<Channel>>} */
	getChannels() {
		return this._postRequest({
			query: "channels",
		});
	}

	/** @returns {Promise<number>} */
	getSession() {
		return this._postRequest({
			query: "session",
		});
	}
}

export async function setupMumbleWorker(recorderSab, playbackSab) {
	const worker = new Worker(new URL("worker.js", import.meta.url), {
		type: "module",
	});

	const msg = await waitForAnyMessage(worker);
	if (msg.data !== "init") {
		const err = `expected init message from mumble worker, got: ${msg.data}`;
		throw err;
	}

	worker.postMessage({
		command: "init",
		recorderSab,
		playbackSab,
	});

	return worker;
}

/** @typedef{import("/js/native/speakez/speakez_web.js").User} User */
/** @type{User} */
export let User;

/** @typedef{import("/js/native/speakez/speakez_web.js").Channel} Channel */
/** @type{Channel} */
export let Channel;

/** @type{import("/js/native/speakez/speakez_web.js").Event} */
export let MumbleEvent;

/** @type{import("/js/native/speakez/speakez_web.js").Response} */
export let Response;

/** @type{import("/js/native/speakez/speakez_web.js").Connect} */
export let ConnectResponse;
