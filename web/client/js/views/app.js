import { html } from "html";
import { createContext } from "preact";
import { useContext, useState, useErrorBoundary } from "preact/hooks";

import { Router } from "./router.js";
import { ServerList, ServerView, Settings } from "./index.js";
import { State } from "../state.js";
import Navbar from "./navbar.js";

/** @type {import("preact").Context<State>} */
const StateProvider = createContext();

/** @type {import("preact").Context<(any) => void>} */
const DispatchProvider = createContext();

export const useAppState = () => useContext(StateProvider);

/**
 * @returns {[(Object, bool) => void,any,any]} deps
 */
export const useDispatch = (wantResult = false) => {
	const dispatcher = useContext(DispatchProvider);
	const [result, setResult] = useState(null);
	const [error, setError] = useState(null);

	const dispatch = (msg) => {
		let result = Promise.resolve(dispatcher(msg));
		if (wantResult) {
			result = result.then(setResult);
		}
		result.catch(setError);
	};

	return [dispatch, result, error];
};

/**
 * @param {Object} props
 * @param {State} props.state
 */
export function App({ state, dispatch }) {
	const [_error, _resetError] = useErrorBoundary((error) =>
		console.error("app error boundary", error.message),
	);

	return html`
<${StateProvider.Provider} value=${state}>
    <${DispatchProvider.Provider} value=${dispatch}>
        <${Navbar} />
        <main>
            <${Router}>
                <${ServerList} path=/ default />
                <${ServerView} path=/servers/:name />
                <${Settings} path=/settings />
            </Router>
        </main>
    </DispatchProvider>
</StateProvider>
`;
}

export default App;
