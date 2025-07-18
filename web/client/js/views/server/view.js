import { html } from "html";
import { useEffect } from "preact/hooks";
import { useTrackRenderCount } from "../hooks.js";

import { useAppState, useDispatch } from "../app.js";
import { Redirect } from "../router.js";
import Channels from "./channels.js";
import Messages from "./messages.js";

import { actions } from "../../state.js";

/**
 * @param {Object} props
 */
const ServerView = ({ params }) => {
	DEBUG && useTrackRenderCount(`server-view-${params.name}`);

	const state = useAppState();
	const server = state.servers.peek().find((s) => s.name === params.name);
	const [dispatch, _, error] = useDispatch(true);

	if (server === undefined) {
		return html`<${Redirect} to=/ />"`;
	}

	if (error) {
		return html`<div>Error connecting to server (${server.addr}): ${error.message}</div>`;
	}

	const needsToConnect = server.id !== state.active_server.id.peek();
	if (needsToConnect) {
		const msg = actions.connect(server.id, {
			name: "test_user",
			pass: "",
		});
		useEffect(() => dispatch(msg));

		return html`<div>Connecting to server...</div>`;
	}

	// biome-ignore format:
	return html`
    <div class="flex flex-row h-full items-center">
        <${Messages} />
        <${Channels} />
    </div>
`;
};

export default ServerView;
