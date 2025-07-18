import { signal, Signal, computed, batch } from "@preact/signals";
import { dynamicSort, runWithTimeout } from "./lib/util.js";
import { Channel, User } from "./mumble/index.js";

/** @type {State} */
export let State;

/**
 * @typedef {ReturnType<typeof createState>} InternalState
 */

/**
 * @typedef {ReturnType<typeof createStateWithViews>} State
 */

let idVal = 1;
const nextID = () => {
	return idVal++;
};

export const createState = () => {
	const s = {
		servers: signal([
			{
				id: nextID(),
				name: "Server1",
				addr: "/ws",
			},
			{
				id: nextID(),
				name: "Server2",
				addr: "wss://localhost:/ws",
			},
			{
				id: nextID(),
				name: "Server3",
				addr: "",
			},
		]),
		active_server: {
			session: signal(0),

			id: signal(0),
			/** @type {Signal<Array<import("./mumble/index.js").Channel>>} */
			channels: signal([]),
			/** @type {Signal<Array<import("./mumble/index.js").User>>} */
			users: signal([]),

			/** @type {Signal<Set<number>>} */
			users_talking: signal(new Set()),

			/** @type {Signal<Array<string>>} */
			messages: signal([]),
		},
		mic: {
			muted: signal(false),
			monitoring: signal(false),
			outputGain: signal(0),
			volumeLevel: signal(0),
		},
	};
	return s;
};

/** @param {InternalState} s */
export function createStateWithViews() {
	const s = createState();
	const views = viewsFromState(s);

	return {
		views,
		...s,
	};
}

/** @typedef {ReturnType<typeof channelView>} ChannelView */
/** @typedef {ReturnType<typeof userView>} UserView */

/**
 * @param {Channel} channel
 */
function channelView(channel) {
	return {
		...channel,
		position: channel.position || 0,
		/** @type {Array<UserView>} */
		users: [],
		subchannels: [],
	};
}

/**
 * @param {User} user
 */
function userView(user) {
	return {
		...user,
		talking: false,
	};
}

/**
 * @param {Array<UserView>} users
 * @param {Array<Channel>} channels
 */
function combineChannelsAndUsers(channels, users) {
	/** @type {Map<number,ChannelView>} */
	const channelsByID = new Map();
	for (const channel of channels) {
		channelsByID.set(channel.id, channelView(channel));
	}

	for (const user of users) {
		const c = channelsByID.get(user.channel);
		if (c) {
			c.users.push(user);
		}
	}

	for (const c of channelsByID.values()) {
		c.users.sort(dynamicSort("name"));
	}

	return channelsByID;
}
/**
 * @param {Array<User>} users
 * @param {Set<number>} usersTalking
 */
function transformUsers(users, usersTalking) {
	return users.map(userView).map((u) => {
		u.talking = usersTalking.has(u.session);
		return u;
	});
}

/** @param {InternalState} state */
export function viewsFromState(state) {
	const users = computed(() => {
		const usersTalking = state.active_server.users_talking.value;
		return transformUsers(state.active_server.users.value, usersTalking);
	});

	const currentUser = computed(() => {
		const session = state.active_server.session.value;
		return users.value.find((u) => u.session === session);
	});

	const channels = computed(() => {
		const channelsMap = combineChannelsAndUsers(
			state.active_server.channels.value,
			users.value,
		);

		for (const channel of channelsMap.values()) {
			const parent = channelsMap.get(channel.parent);
			if (parent === undefined || parent === null) {
				continue;
			}

			parent.subchannels.push(channel);
			parent.subchannels = parent.subchannels.sort(dynamicSort("-position"));
		}

		const rootChannels = Array.from(channelsMap.values())
			.filter((c) => c.parent === undefined || c.parent === null)
			.sort(dynamicSort("-position"));
		return rootChannels;
	});

	return {
		currentUser,
		users,
		channels,
		active_server: computed(() => {
			const id = state.active_server.id.value;
			return {
				name: state.servers.value.find((s) => s.id === id)?.name,
			};
		}),
	};
}

/**
 * @typedef {Object} StateAndWorker
 * @property {State} state
 * @property {import("./mumble/client.js").Client} mumbleWorker
 */

/**
 * @typedef {Object} StateAndMic
 * @property {State} state
 */

/**
 * @param {StateAndWorker} o
 * @param {ReturnType<typeof actions.connect>} msg
 */
function handleConnect({ state, mumbleWorker }, msg) {
	state.active_server.id.value = 0;
	const server = state.servers.value.find((s) => s.id === msg.serverID);
	const c = {
		url: server.addr,
		user: msg.auth.name,
		pass: msg.auth.pass,
	};

	return runWithTimeout(
		mumbleWorker.connect(c).then((result) => {
			batch(() => {
				console.log("connected", result);
				state.active_server.id.value = msg.serverID;
				state.active_server.session.value = result.session;
				state.active_server.channels.value = result.channels;
				state.active_server.users.value = result.users;
			});
		}),
		1000,
	);
}

/**
 * @param {StateAndWorker} o
 * @param {ReturnType<typeof actions.switchChannel>} msg
 */
function handleSwitchChannel({ state, mumbleWorker }, msg) {
	return mumbleWorker.switchChannel(msg.channelID);
}

/**
 * @param {StateAndWorker} o
 */
function handleDiconnect({ state, mumbleWorker }) {
	if (window.location.toString().includes("#!/servers/")) {
		// Use replace to avoid history entry
		window.location.replace("#!/");
	}
	// while on the server-view page if this takes place immediately it can cause a loop
	setTimeout(() => mumbleWorker.disconnect(), 100);

	batch(() => {
		state.active_server.id.value = 0;
		state.active_server.channels.value = [];
		state.active_server.users.value = [];
	});
}

/**
 * @param {StateAndMic} o
 * @param {ReturnType<typeof actions.micMute>} msg
 */
function handleMicMute({ state }, msg) {
	state.mic.muted.value = msg.value;
	state.mic.volumeLevel.value = 0;
}

/**
 * @param {StateAndMic} o
 * @param {ReturnType<typeof actions.micMonitor>} msg
 */
function handleMicMonitor({ state }, msg) {
	state.mic.monitoring.value = msg.value;
}

/**
 * @param {StateAndMic} o
 * @param {ReturnType<typeof actions.micCalulateVolumeLevel>} msg
 */
function handleMicVolume({ state, mic }) {
	const level = mic.calculateVolumeLevel();
	state.mic.volumeLevel.value = level;
}

/**
 * Enum action types.
 * @readonly
 * @enum {string}
 */
const ActionTypes = {
	CONNECT: "CONNECT",
	DISCONNECT: "DISCONNECT",
	MIC_CALCULATE_VOLUME_LEVEL: "MIC_CALCULATE_VOLUME_LEVEL",
	MIC_MUTE: "MIC_MUTE",
	MIC_MONITOR: "MIC_MONITOR",
	SWITCH_CHANNEL: "SWITCH_CHANNEL",
};

/**
 * @param {State} state
 * @param {Object} o
 * @param {import('./mumble/client.js').Client} o.mumbleWorker
 * @param {import('./audio/voice.js').Mic} o.mic
 */
export function newDispatch(state, { mumbleWorker, mic }) {
	/** @param {action} action */
	return ({ type, ...msg }) => {
		switch (type) {
			case ActionTypes.CONNECT: {
				return handleConnect({ state, mumbleWorker }, msg);
			}
			case ActionTypes.DISCONNECT: {
				return handleDiconnect({ state, mumbleWorker });
			}
			case ActionTypes.SWITCH_CHANNEL: {
				return handleSwitchChannel({ state, mumbleWorker }, msg);
			}
			case ActionTypes.MIC_MUTE: {
				return handleMicMute({ state }, msg);
			}
			case ActionTypes.MIC_MONITOR: {
				return handleMicMonitor({ state }, msg);
			}
			case ActionTypes.MIC_CALCULATE_VOLUME_LEVEL: {
				return handleMicVolume({ state, mic });
			}
		}
	};
}

function updateUser(state, userID, fn) {
	const users = state.active_server.users.value;
	const newUsers = users.map((u) => {
		if (u.session === userID) {
			return fn(u);
		}
		return u;
	});
	state.active_server.users.value = newUsers;
}

/**
 * @typedef {Object} UserStartedTalking
 * @property {string} type
 * @property {Object} data
 * @property {import("./native/speakez/speakez_web.js").Session} data.session
 */

/**
 * @param {State} state
 */
export function newMumbleReducer(state) {
	/** @param {import("./mumble/index.js").MumbleEvent} e */
	return ({ type, data: msg }) => {
		switch (type) {
			case "UserSwitchedChannel": {
				updateUser(state, msg.user, (u) => {
					return { ...u, channel: msg.to_channel };
				});
				return;
			}
			case "UserRemoved": {
				const users = state.active_server.users.value;
				const newUsers = users.filter((u) => u.session !== msg.user);
				state.active_server.users.value = newUsers;
				return;
			}
			case "UserJoinedServer": {
				const user = {
					channel: msg.channel_id,
					name: msg.name,
					session: msg.user,
				};
				const users = state.active_server.users.value;
				state.active_server.users.value = [user, ...users];
				return;
			}
			case "UserStoppedTalking": {
				const value = state.active_server.users_talking.value;
				value.delete(msg.session);
				state.active_server.users_talking.value = new Set(value);
				return;
			}
			case "UserStartedTalking": {
				const value = state.active_server.users_talking.value;
				value.add(msg.session);
				state.active_server.users_talking.value = new Set(value);
				return;
			}
			case "UserSentMessage": {
				const messages = state.active_server.messages.value;
				state.active_server.messages.value = [...messages, msg.message];
				return;
			}
		}
	};
}

/**
 * @typedef {ReturnType<typeof actions[keyof typeof actions]>} action
 */

export const actions = {
	micCalulateVolumeLevel: () => {
		return {
			type: ActionTypes.MIC_CALCULATE_VOLUME_LEVEL,
		};
	},
	micMonitor: (value) => {
		return {
			type: ActionTypes.MIC_MONITOR,
			value,
		};
	},
	micMute: (value) => {
		return {
			type: ActionTypes.MIC_MUTE,
			value,
		};
	},
	connect: (serverID, auth) => {
		return {
			type: ActionTypes.CONNECT,
			serverID,
			auth,
		};
	},
	disconnect: () => {
		return {
			type: ActionTypes.DISCONNECT,
		};
	},
	switchChannel(channelID) {
		return {
			type: ActionTypes.SWITCH_CHANNEL,
			channelID,
		};
	},
};
