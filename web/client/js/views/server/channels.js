import { html } from "html";
import { useTrackRenderCount } from "../hooks.js";

import { useAppState, useDispatch } from "../app.js";
import User from "./user.js";

import { actions } from "/js/state.js";

/**
 * @param {Object} props
 * @param {import("../../state.js").ChannelView} props.channel
 */
const Channel = ({ channel: c }) => {
	DEBUG && useTrackRenderCount(`channel-${c.name}`);

	const state = useAppState();
	const [dispatch] = useDispatch();

	const currentUser = state.views.currentUser.value;
	const onDblClick = (e) => {
		const channelID = Number.parseInt(e.srcElement.dataset.id, 10);

		if (currentUser.channel !== channelID) {
			dispatch(actions.switchChannel(channelID));
		}
	};

	const onCtxMenu = (e) => {
		e.preventDefault();

		const channelID = Number.parseInt(e.srcElement.dataset.id, 10);
		console.log("right clicked channel: ", channelID);
	};

	const clickable = currentUser.channel !== c.id ? "clickable" : "";

	// biome-ignore format:
	return html`
    <li class="channel">
        <h3 class="${clickable}" ondblclick=${onDblClick} oncontextmenu="${onCtxMenu}" data-id=${c.id}>${c.name}</h3>
        <ol aria-label="users" class="users">
            ${c.users?.map(u => html`
                <${User} key=${u.session} user=${u} />
            `)}
        </ol>
        <ol aria-label="subchannels" class="subchannels">
            ${c.subchannels?.map(c => html`
                <${Channel} key=${c.id} channel=${c} />
            `)}
        </ol>
    </li>
`;
};

const Channels = () => {
	DEBUG && useTrackRenderCount("channels");

	const state = useAppState();
	const channels = state.views.channels.value;

	// biome-ignore format:
	return html`
   <ol aria-label="rootChannels" class="channels h-full">
       ${channels.map(c => html`
           <${Channel} key=${c.id} channel=${c} />
       `)}
   </ol>
`;
};

export default Channels;
