pub struct Permissions;

pub fn default() -> u32 {
    Permissions::TRAVERSE
        | Permissions::ENTER
        | Permissions::SPEAK
        | Permissions::LISTEN
        | Permissions::TEXT_MESSAGE
        | Permissions::WHISPER
}

// enum Perm {
// 		None            = 0x0,
// 		Write           = 0x1,
// 		Traverse        = 0x2,
// 		Enter           = 0x4,
// 		Speak           = 0x8,
// 		MuteDeafen      = 0x10,
// 		Move            = 0x20,
// 		MakeChannel     = 0x40,
// 		LinkChannel     = 0x80,
// 		Whisper         = 0x100,
// 		TextMessage     = 0x200,
// 		MakeTempChannel = 0x400,
// 		Listen          = 0x800,

// 		// Root channel only
// 		Kick             = 0x10000,
// 		Ban              = 0x20000,
// 		Register         = 0x40000,
// 		SelfRegister     = 0x80000,
// 		ResetUserContent = 0x100000,

// 		Cached = 0x8000000,
// 		All = Write + Traverse + Enter + Speak + MuteDeafen + Move + MakeChannel + LinkChannel + Whisper + TextMessage
// 			  + MakeTempChannel + Listen + Kick + Ban + Register + SelfRegister + ResetUserContent
// 	};

impl Permissions {
    const NONE: u32 = 0;
    /// Write access to channel control. Implies all other permissions (except Speak).
    const WRITE: u32 = 0x01;
    /// Traverse channel.
    /// Without this, a client cannot reach subchannels, no matter which privileges it has there.
    const TRAVERSE: u32 = 0x02;
    /// Enter channel.
    const ENTER: u32 = 0x04;
    /// Speak in channel.
    const SPEAK: u32 = 0x08;
    /// Whisper to channel. This is different from Speak, so you can set up different permissions.
    const WHISPER: u32 = 0x100;
    /// Send text message to channel.
    const TEXT_MESSAGE: u32 = 0x200;
    const LISTEN: u32 = 0x800;
}
// 	/** Mute and deafen other users in this channel. */
// 	const int PermissionMuteDeafen = 0x10;
// 	/** Move users from channel. You need this permission in both the source and destination channel to move another user. */
// 	const int PermissionMove = 0x20;
// 	/** Make new channel as a subchannel of this channel. */
// 	const int PermissionMakeChannel = 0x40;
// 	/** Make new temporary channel as a subchannel of this channel. */
// 	const int PermissionMakeTempChannel = 0x400;
// 	/** Link this channel. You need this permission in both the source and destination channel to link channels, or in either channel to unlink them. */
// 	const int PermissionLinkChannel = 0x80;
// 	/** Kick user from server. Only valid on root channel. */
// 	const int PermissionKick = 0x10000;
// 	/** Ban user from server. Only valid on root channel. */
// 	const int PermissionBan = 0x20000;
// 	/** Register and unregister users. Only valid on root channel. */
// 	const int PermissionRegister = 0x40000;
// 	/** Register and unregister users. Only valid on root channel. */
// 	const int PermissionRegisterSelf = 0x80000;
