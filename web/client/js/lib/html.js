import htm from "htm";
import { h, Fragment } from "preact";

const registry = {
	"": Fragment,
};

export function register(key, value) {
	registry[key] = value;
}

function createElement(tag, props, ...children) {
	const component = registry[tag] || tag;
	return h(component, props, ...children);
}
export const html = htm.bind(createElement);

export { h, render } from "preact";
