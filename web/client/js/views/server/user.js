import { html } from "html";
import { useTrackRenderCount } from "../hooks.js";

import { useAppState } from "../app.js";

/**
 * @param {Object} props
 * @param {import("../../state.js").UserView} props.user
 */
const User = ({ user: u }) => {
	DEBUG && useTrackRenderCount(`user-${u.name}`);

	const currentUser = useAppState().views.currentUser.value;
	const you = currentUser.session === u.session ? "you" : "";
	const talking = u.talking ? "talking" : "";

	// biome-ignore format:
	return html`
    <li class="user ${you} ${talking}">
        ${u.name}
    </li>
`;
};

export default User;
