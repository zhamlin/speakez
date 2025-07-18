import { html } from "html";
import { useTrackRenderCount } from "../hooks.js";
import { useAppState } from "../app.js";

// TODO: Show ping and player count?

/**
 */
const ServerList = () => {
	DEBUG && useTrackRenderCount("server-list");

	const servers = useAppState().servers;

	// biome-ignore format:
	return html`
<div id="server-list" class="flex flex-col items-center h-full">
    <div class="flex flex-col justify-center flex-[2]">
        <ul role="list" class="overflow-y-scroll">
            ${servers.value.map((s) => html`
                <li key=${s.id} class="text-xl even:bg-slate-100 odd:bg-white">
                    <a class="clickable" href="#!/servers/${s.name}">${s.name}</a>
                </li>
            `)}
        </ul>
    </div>

    <div class="flex flex-col flex-[1] container py-1 px-1 items-center">
        <button class="bg-gray-300 hover:bg-gray-400 text-gray-800 font-bold py-2 px-4 rounded w-1/3 sm:w-1/6"
            input="button">
            Add
        </button>
    </div>
</div>`;
};

export default ServerList;
