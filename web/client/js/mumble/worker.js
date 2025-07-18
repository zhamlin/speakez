import { RingBuffer } from "/js/third_party/ringbuf/index.js";

import { ResumableReader, writeSizedMessage } from "/js/lib/ringbuf.js";

import speakez_init, {
	new_message_buf,
	handle_message,
	version_message,
	tunneled_opus_message,
	StateWrapper,
	new_connection,
	authenticate_message,
	switch_channel,
} from "/js/native/speakez/speakez_web.js";

await speakez_init({})
	.then(() => console.log("speakez wasm loaded"))
	.catch((e) => {
		postMessage(e);
	});

/**
 * @typedef {Object} WorkerDeps
 * @property {Worker} audioWorker
 * @property {Worker} writeBuffer
 * @property {RingBuffer} audioWriter
 * @property {Object} auth
 * @property {String} auth.name
 * @property {String} auth.pass
 * Setup and start the event loop for the mumble connection.
 */

/** @type {ReturnType<typeof newState>} */
let state = newState();

function newState() {
	return {
		/** @type {WebSocket} */
		ws: null,

		/** @type {ResumableReader} */
		recorder_reader: null,

		recorder_interval: 0,

		frameNumber: BigInt(0),

		/** @type {RingBuffer} */
		playback_writer: null,

		/** @type {StateWrapper} */
		mumble: null,

		/** current users session */
		session: 0,

		/** @type {Map<number, number>} */
		users_talking: new Map(),
	};
}

async function init(input) {
	const recorder_ringbuf = new RingBuffer(input.recorderSab, Uint8Array);
	const staging_buffer = new Float32Array(4096 * 4);
	state.recorder_reader = new ResumableReader(recorder_ringbuf, staging_buffer);

	state.playback_writer = new RingBuffer(input.playbackSab, Uint8Array);
}

function switchChannel({ _tag, channelId }) {
	const data = switch_channel(state.mumble, channelId);
	state.ws.send(data);
	sendTaggedMessage(_tag, {});
}

function resetState() {
	state = {
		...newState(),
		recorder_reader: state.recorder_reader,
		playback_writer: state.playback_writer,
	};
}

function disconnect() {
	if (state.recorder_interval) {
		clearInterval(state.recorder_interval);
	}

	if (state.ws) {
		state.ws.close();
	}

	resetState();
}

function connect_to_server({ _tag, url, user, pass }) {
	disconnect();

	const ws = new WebSocket(url);
	ws.binaryType = "arraybuffer";
	state.ws = ws;

	setup_websocket(ws, {
		auth: {
			name: user,
			pass,
		},
		_tag,
	});
}

function handleUserTalking(session) {
	const talkingTimeout = state.users_talking.get(session);
	if (talkingTimeout) {
		clearTimeout(talkingTimeout);
	} else {
		postMessage({
			type: "UserStartedTalking",
			data: {
				session,
			},
		});
	}

	const timeout = setTimeout(() => {
		state.users_talking.delete(session);
		postMessage({ type: "UserStoppedTalking", data: { session } });
	}, 100);
	state.users_talking.set(session, timeout);
}

function readFromMicQueue() {
	let msg = state.recorder_reader.readSizedMessage();

	while (msg) {
		// if (!writeSizedMessage(msg, playback_writer)) {
		// 	console.log("mumble can not write data, audio processor may be behind");
		// }

		const audio_message = tunneled_opus_message(
			state.session,
			state.frameNumber++,
			msg,
			false,
		);
		handleUserTalking(state.session);
		state.ws.send(audio_message);

		msg = state.recorder_reader.readSizedMessage();
	}
}

function sendTaggedMessage(tag, msg) {
	postMessage({
		_tag: tag,
		data: msg,
	});
}

function waitForRustState() {
	return new Promise((resolve) => {
		let checkForValue;
		const fin = () => {
			if (state.mumble) {
				clearInterval(checkForValue);
				resolve(state.mumble);
				return true;
			}
		};

		if (fin() !== true) {
			checkForValue = setInterval(fin, 200);
		}
	});
}

function handleCommand(data) {
	switch (data.command) {
		case "init": {
			init(data);
			return;
		}
		case "connect": {
			const alreadyConnected = state.ws && state.ws.url === data.addr;
			if (alreadyConnected) {
				waitForRustState().then(() => sendConnected(data._tag, state.mumble));
				return;
			}

			connect_to_server(data);
			return;
		}
		case "disconnect": {
			disconnect();
			return;
		}
		case "switch_channel": {
			switchChannel(data);
			return;
		}
	}
}

/** @param {StateWrapper} state */
function handleQuery(state, data) {
	switch (data.query) {
		case "channel": {
			return state.channel(data.id);
		}
		case "channels": {
			return state.channels();
		}
		case "user": {
			return state.user(data.id);
		}
		case "users": {
			return state.users();
		}
		case "session": {
			return state.session();
		}
	}
}

/**
 * @param {MessageEvent} e
 */
self.onmessage = (e) => {
	if (e.data.command) {
		handleCommand(e.data);
	}

	if (!state.mumble) {
		return;
	}

	if (e.data.query) {
		const msg = handleQuery(state.mumble, e.data);
		if (msg) {
			sendTaggedMessage(e.data._tag, msg);
		} else {
			console.log(`query had no msg: ${e.data.query}`);
		}
		return;
	}
};

/**
 * @param {string} tag
 * @param {StateWrapper} state
 */
function sendConnected(tag, state) {
	const msg = {
		command: "connect",
		users: state.users(),
		channels: state.channels(),
		session: state.session(),
	};
	sendTaggedMessage(tag, msg);
}

/**
 * @param {WebSocket} conn
 * @param {WorkerDeps} deps
 * Setup and start the event loop for the mumble connection.
 */
function setup_websocket(conn, deps) {
	conn.onopen = () => {
		const version = version_message();
		conn.send(version);
	};

	conn.onclose = (e) => {};
	conn.onerror = (e) => {
		console.error("error from mumble websocket", e);
	};

	/** @param {StateWrapper} state */
	const onHandshakeComplete = (rustState) => {
		state.mumble = rustState;
		state.recorder_interval = setInterval(readFromMicQueue, 30);
		state.session = rustState.session();
		sendConnected(deps._tag, rustState);
	};

	let message_handler = new_handshake_handle_message(
		conn,
		deps,
		onHandshakeComplete,
	);

	/**
	 * @param {MessageEvent<ArrayBuffer>} e
	 */
	conn.onmessage = (e) => {
		const bytes = new Uint8Array(e.data);
		const message_buf = new_message_buf(bytes);

		if (message_buf === undefined) {
			console.error("received invalid mumble message from websocket");
			return;
		}

		const result = message_handler(message_buf);
		if (result) {
			message_handler = result;
		}
	};
}

/**
 * @param {WebSocket} conn
 * @param {Object} auth
 * @param {String} auth.name
 * @param {String} auth.pass
 */
function new_handshake_handle_message(conn, deps, onComplete) {
	let handshake = new_connection();

	return (message_buf) => {
		const result = handshake.handle_message(message_buf);

		if (result.is_connected()) {
			const state = result.to_connected();
			onComplete(state);
			return create_connected_handle_message(deps);
		}

		handshake = result.to_handshake();
		if (handshake.should_send_authenticate()) {
			const msg = authenticate_message(deps.auth.name, deps.auth.pass);
			conn.send(msg);
			handshake.sent_authenticate();
		}
	};
}

/**
 * @param {StateWrapper} state
 * @param {WorkerDeps} deps
 */
function create_connected_handle_message(deps) {
	return (message_buf) => {
		state.mumble = handle_message(state.mumble, message_buf);

		let event = state.mumble.next_event();
		while (event) {
			handle_event(deps, event);
			event = state.mumble.next_event();
		}
	};
}

/**
 * @param {WorkerDeps} deps
 * @param {import("./index.js").MumbleEvent} event
 */
function handle_event(deps, event) {
	if (event.type === "UserSentAudio") {
		const data = new Uint8Array(event.data.data);

		if (!writeSizedMessage(data, state.playback_writer)) {
			console.log("mumble can not write data, audio processor may be behind");
		}

		handleUserTalking(event.data.sender);
		return;
	}

	// forward message to main thread
	postMessage(event);
}

postMessage("init");
