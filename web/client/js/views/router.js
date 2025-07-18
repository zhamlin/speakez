import { h } from "html";
import { useSignal } from "@preact/signals";
import { useEffect } from "preact/hooks";

import { useEventListener } from "./hooks.js";

const removePrefix = (value, prefix) =>
	value.startsWith(prefix) ? value.slice(prefix.length) : value;

const stripTrailingSlash = (str) => {
	return str.endsWith("/") ? str.slice(0, -1) : str;
};

const locationHashToPath = (hash) => {
	const path = stripTrailingSlash(removePrefix(hash, "#!"));
	if (path === "") {
		return "/";
	}
	return path;
};

// match paths and extract parameters
const matchPath = (path, route) => {
	const pathParts = path.split("/").filter(Boolean); // split and remove empty strings
	const routeParts = route.split("/").filter(Boolean);

	if (pathParts.length !== routeParts.length) return null;

	const params = {};
	for (let i = 0; i < pathParts.length; i++) {
		if (pathParts[i].startsWith(":")) {
			const paramName = pathParts[i].slice(1);
			params[paramName] = routeParts[i];
		} else if (pathParts[i] !== routeParts[i]) {
			return null;
		}
	}

	return params;
};

const useHashRouter = () => {
	const getPath = () => locationHashToPath(location.hash);
	const currentRoute = useSignal(getPath());

	useEventListener("hashchange", () => {
		currentRoute.value = getPath();
	});

	return currentRoute;
};

export const Router = ({ children }) => {
	const currentRoute = useHashRouter();

	for (const child of children) {
		const { path } = child.props;

		if (path) {
			// TODO: enable forwarding rest of the path?
			// const isMatch = path.startsWith(path);
			// const innerPath = path.substring(path.length);
			const params = matchPath(path, currentRoute.value);
			if (params) {
				return h(child.type, { ...child.props, params });
			}
		}
	}

	return children.find((child) => child.props.default);
};

export const Redirect = ({ to }) => {
	useEffect(() => {
		// window.location.hash = `#!${to}`;
		window.location.replace(`#!${to}`); // Use replace to avoid history entry
	}, [to]);

	return null;
};
