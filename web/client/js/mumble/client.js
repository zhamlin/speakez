export class Client {
	/**
	 * @param {function(string, any): Promise<any>} msg_sender
	 * @param {function(any)} on_message
	 */
	constructor(msg_sender) {
		this._send_msg = msg_sender;
	}

	/**
	 * @returns {Promise<import("/js/native/speakez/speakez_web.js").Connect>>}
	 */
	connect({ url, user, pass }) {
		return this._send_msg("connect", {
			url,
			user,
			pass,
		});
	}

	/**
	 * @returns {Promise<void>>}
	 */
	disconnect() {
		return this._send_msg("disconnect");
	}

	/**
	 * @returns {Promise<void>>}
	 */
	switchChannel(channelID) {
		return this._send_msg("switch_channel", { channelId: channelID });
	}
}
