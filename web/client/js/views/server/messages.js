import { html } from "html";
import { useTrackRenderCount } from "../hooks.js";

import { useAppState } from "../app.js";

/**
 * @param {Object} props
 */
const Messages = () => {
	DEBUG && useTrackRenderCount("messages");

	const state = useAppState();

	const messages = state.active_server.messages.value;
	// biome-ignore format:
	return html`
    <div class="messages h-full">
        <h3>Messages</h3>
        ${messages.map(m => html`
            <div class="message">${m}</div>
        `)}
    </div>
`;
};

export default Messages;
