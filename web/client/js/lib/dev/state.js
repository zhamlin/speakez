import { viewsFromState } from "/js/state.js";
import { wrapNonObjectFieldsWithFn } from "/js/lib/util.js";
import { signal } from "@preact/signals";

function saveState() {
	// window.state.active_server = undefined;
	// window.state.views = undefined;
	// sessionStorage.setItem("state", JSON.stringify(window.state));
}

addEventListener("beforeunload", saveState);
addEventListener("pagehide", saveState);

/**
 * @params {T} fallback
 * @returns {T}
 * @template T
 */
function loadState(fallback) {
	try {
		const state = JSON.parse(sessionStorage.getItem("state"));
		return wrapNonObjectFieldsWithFn(signal, state);
	} catch (err) {
		console.log("error loading state: ", err);
		return fallback;
	}
}

function shouldClearState() {
	const url = new URL(window.location);
	return url.searchParams.get("clear-state");
}

function removeClearStateFlag() {
	const url = new URL(window.location);
	const oldUrl = url;
	const params = url.searchParams;

	params.delete("clear-state");

	// no history
	// window.location = url;

	// allow going back to having the clear state flag
	window.history.pushState({}, "", oldUrl);
}

/**
 * @param {T} state
 * @returns {T}
 * @template T
 */
export function createState(state) {
	if (shouldClearState()) {
		console.log("using new state");
		removeClearStateFlag();
		return state;
	}

	const newState = loadState(state);
	const s = {
		...state,
		...newState,
	};
	s.views = viewsFromState(s);
	return s;
}
