mod env;

use rspotify::{AuthCodeSpotify, Config, Credentials, OAuth, Token};
use serenity::all::{ChannelId, GatewayIntents};

#[cfg(not(debug_assertions))]
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(debug_assertions)]
pub const VERSION: &str = concat!(env!("CARGO_PKG_VERSION"), "-dev");

/// The "listening to" message that shows up under the Spoticord bot user
pub const MOTD: &str = "some good 'ol music";

/// The time it takes (in seconds) for Spoticord to disconnect when no music is being played
// pub const DISCONNECT_TIME: u64 = 5 * 60; // Removed timeout functionality

pub fn discord_token() -> &'static str {
    &env::DISCORD_TOKEN
}

pub fn discord_intents() -> GatewayIntents {
    GatewayIntents::GUILDS | GatewayIntents::GUILD_VOICE_STATES
}

pub fn database_url() -> &'static str {
    &env::DATABASE_URL
}

pub fn link_url() -> &'static str {
    &env::LINK_URL
}

pub fn kv_url() -> &'static str {
    &env::KV_URL
}

pub fn voice_channel_id() -> ChannelId {
    let id_str = &env::DISCORD_VOICE_CHANNEL_ID;
    let id_num: u64 = id_str
        .parse()
        .expect("DISCORD_VOICE_CHANNEL_ID must be a valid u64");
    return ChannelId::from(id_num);
}

pub fn get_spotify(token: Token) -> AuthCodeSpotify {
    AuthCodeSpotify::from_token_with_config(
        token,
        Credentials {
            id: env::SPOTIFY_CLIENT_ID.to_string(),
            secret: Some(env::SPOTIFY_CLIENT_SECRET.to_string()),
        },
        OAuth::default(),
        Config::default(),
    )
}
