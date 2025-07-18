// taken from: https://esbuild.github.io/api/#hot-reloading-css
/** @param {string} filename */
function reloadCss(filename) {
	for (const link of document.getElementsByTagName("link")) {
		const url = new URL(link.href);

		if (url.host === location.host && url.pathname === filename) {
			const next = link.cloneNode();
			if (next instanceof HTMLLinkElement) {
				next.href = `${filename}?${Math.random().toString(36).slice(2)}`;
				next.onload = () => link.remove();
			}
			link.parentNode.insertBefore(next, link.nextSibling);
			console.log("reloading css", filename);
			return;
		}
	}
}

const wsProto = location.protocol === "https:" ? "wss://" : "ws://";
const wsLoc = `${wsProto + location.host}/_/events`;

const conn = new WebSocket(wsLoc);
conn.onerror = (err) => {
	console.error(err);
};

conn.onopen = () => {
	console.log("reload ws connected", wsLoc);
};

/** @param {MessageEvent} m **/
conn.onmessage = (m) => {
	const msg = JSON.parse(m.data);
	const filename = msg.data;
	const ext = filename.split(".").pop();
	const isCss = ext === "css";

	if (isCss) {
		reloadCss(`${filename}`);
	} else {
		location.reload();
	}
};

window.onbeforeunload = () => {
	console.log("closing websocket");
	conn.close();
};
