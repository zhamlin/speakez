import { html, render } from "html";

import { App } from "./views/app.js";
import { createState } from "./lib/dev/state.js";
import {
	createStateWithViews as createAppState,
	newDispatch,
	newMumbleReducer,
} from "./state.js";

import { Client } from "./mumble/client.js";
import { effect } from "@preact/signals";

window.DEBUG = true;

function featureDetection() {
	return {
		hasTauri: typeof __TAURI__ !== "undefined",
	};
}

/**
 * @param {ReturnType<typeof featureDetection>} features
 */
function confirmFeatureSupport(features) {
	if (!features.hasTauri) {
		alert("Tauri support is required");
	}
}

const features = featureDetection();
DEBUG && console.log(features);
confirmFeatureSupport(features);

/** @type {import("@tuari/api")} */
const tauri = window.__TAURI__;

const client = new Client((msg, data) => {
	return tauri.core.invoke(msg, data);
});
client.disconnect();

const state = createState(createAppState());

effect(() => {
	tauri.core.invoke("mic_mute", { value: state.mic.muted.value });
});
effect(() => {
	tauri.core.invoke("mic_monitor", { value: state.mic.monitoring.value });
});

tauri.core.invoke("set_input");
tauri.core.invoke("set_output");

window.state = state;
state.servers.value[0].addr = "127.0.0.1:64738";

const mumbleReducer = newMumbleReducer(state);

tauri.event.listen("mumble", (event) => {
	DEBUG && console.log(event);
	mumbleReducer(event.payload);
});

const dispatch = newDispatch(state, { mumbleWorker: client, mic: {} });
window.dispatch = dispatch;

const body = document.querySelector("body");
render(html`<${App} state=${state} dispatch=${dispatch} />`, body);
