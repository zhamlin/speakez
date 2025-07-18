import { html } from "html";

import { useAppState, useDispatch } from "./app.js";
import { actions } from "../state.js";

import { useInterval, useTrackRenderCount } from "./hooks.js";

const Settings = () => {
	DEBUG && useTrackRenderCount("settings");

	const mic = useAppState().mic;
	const [dispatch] = useDispatch();

	const onMuteClick = () => {
		const newValue = !mic.muted.value;
		dispatch(actions.micMute(newValue));
	};

	const onMonitorClick = () => {
		const newValue = !mic.monitoring.value;
		dispatch(actions.micMonitor(newValue));
	};

	useInterval(() => {
		if (!mic.muted.value) {
			dispatch(actions.micCalulateVolumeLevel());
		}
	}, 50);

	return html`
<div class="flex flex-col items-center">
    <h2>Voice Settings</h2>
    <fieldset>
        <legend>Mic Settings:</legend>

        <div>
            <label>
                <input type="checkbox" checked=${mic.monitoring} onclick=${onMonitorClick}/>
                Monitor
            </label>
        </div>

        <div>
            <label>
                <input type="checkbox" id="mute" checked=${mic.muted} onclick=${onMuteClick}/>
                Mute
            </label>
        </div>

        <br/>
        <div>
            <label class="flex flex-col items-center">
                <span>Volume</span>
                <meter name="volume" class="w-full" min="0" low="30" high="70" max="100" value=${mic.volumeLevel}></meter>
            </label>
        </div>
    </fieldset>
</div>`;
};

export default Settings;
