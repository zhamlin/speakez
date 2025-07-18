/**
 * A session represents a unique ID for a given user.
 */
export type Session = number;
/**
 * Only opus is supported.
 */
export type VoiceMessage = {
	data: number[];
	frame_number: number;
	sender: Session;
};
export type Channel = {
	description: string;
	id: ChannelID;
	max_users?: number;
	name: string;
	parent?: ChannelID;
	position?: number;
	temporary: boolean;
};
export type ChannelID = number;
export type Connect = {
	channels: Channel[];
	session: Session;
	users: User[];
};
export type Disconnect = null;
export type Event =
	| {
			data: VoiceMessage;
			type: "UserSentAudio";
	  }
	| {
			data: UserSentMessage;
			type: "UserSentMessage";
	  }
	| {
			data: UserRemoved;
			type: "UserRemoved";
	  }
	| {
			data: UserSwitchedChannel;
			type: "UserSwitchedChannel";
	  }
	| {
			data: UserJoinedServer;
			type: "UserJoinedServer";
	  };
export type Response =
	| {
			data: Connect;
			type: "Connect";
	  }
	| {
			data: Disconnect;
			type: "Disconnect";
	  };
export type User = {
	channel: ChannelID;
	name: string;
	session: Session;
};
export type UserJoinedServer = {
	channel_id: ChannelID;
	name: string;
	/**
	 * The user who joined.
	 */
	user: Session;
};
export type UserRemoved = {
	reason: UserRemovedReason;
	reason_msg?: string;
	/**
	 * the user who was removed.
	 */
	user: Session;
};
export type UserRemovedReason =
	| "Left"
	| {
			Kicked: {
				/**
				 * The user who initiated the removal.
				 */
				by: Session;
			};
	  }
	| {
			Banned: {
				/**
				 * The user who initiated the removal.
				 */
				by: Session;
			};
	  };
export type UserSentMessage = {
	/**
	 * Channels that should receive the message.
	 */
	channels: ChannelID[];
	message: string;
	/**
	 * Users who should receive them message.
	 */
	recipients: Session[];
	/**
	 * The user who sent the message.
	 */
	user: Session;
};
export type UserSwitchedChannel = {
	from_channel: ChannelID;
	to_channel: ChannelID;
	/**
	 * The user who switched channels.
	 */
	user: Session;
};
