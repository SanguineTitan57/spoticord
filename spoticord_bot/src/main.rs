mod bot;
mod commands;

use anyhow::Context as _;
use log::{error, info};
use poise::Framework;
use serenity::all::ClientBuilder;
use shuttle_runtime::SecretStore;
use songbird::SerenityInit;
use spoticord_database::Database;
use std::env;
use std::result::Result::Ok;

#[shuttle_runtime::main]
async fn main(
    #[shuttle_runtime::Secrets] secrets: SecretStore,
) -> shuttle_serenity::ShuttleSerenity {
    // Force aws-lc-rs as default crypto provider
    // Since multiple dependencies either enable aws_lc_rs or ring, they cause a clash, so we have to
    // explicitly tell rustls to use the aws-lc-rs provider
    _ = rustls::crypto::aws_lc_rs::default_provider().install_default();

    // Setup logging
    if std::env::var("RUST_LOG").is_err() {
        #[cfg(debug_assertions)]
        std::env::set_var("RUST_LOG", "spoticord");

        #[cfg(not(debug_assertions))]
        std::env::set_var("RUST_LOG", "spoticord=info");
    }

    // Shuttle runtime already installs a global tracing/logging subscriber.
    // Using init() after another logger is set causes a panic (SetLoggerError).
    // try_init() will silently ignore if a logger is already installed.
    // Remove or comment out the line below if you want to use shuttle's default logging.
    // let _ = env_logger::init();

    info!("Today is a good day!");
    info!(" - Spoticord");

    let _ = dotenvy::dotenv().ok();

    // Get the discord token set in `Secrets.toml`
    let discord_token = secrets
        .get("DISCORD_TOKEN")
        .context("'DISCORD_TOKEN' was not found")?;
    let database_url = secrets
        .get("DATABASE_URL")
        .context("'DATABASE_URL' was not found")?;
    let link_url = secrets
        .get("LINK_URL")
        .context("'LINK_URL' was not found")?;
    let spotify_client_id = secrets
        .get("SPOTIFY_CLIENT_ID")
        .context("Missing SPOTIFY_CLIENT_ID");
    let spotify_client_secret = secrets
        .get("SPOTIFY_CLIENT_SECRET")
        .context("Missing SPOTIFY_CLIENT_SECRET");

    // Optional
    let guild_id = secrets.get("GUILD_ID");

    // --- Set environment variables for spoticord_config ---
    env::set_var("DISCORD_TOKEN", &discord_token);
    env::set_var("DATABASE_URL", &database_url);
    env::set_var("LINK_URL", &link_url);
    env::set_var("SPOTIFY_CLIENT_ID", &spotify_client_id?);
    env::set_var("SPOTIFY_CLIENT_SECRET", &spotify_client_secret?);

    // Set optional environment variables if they exist
    if let Some(guild_id) = guild_id {
        env::set_var("GUILD_ID", guild_id);
    }

    // Set up database
    let database: Database = match Database::connect().await {
        Ok(db) => db,
        Err(why) => {
            error!("Failed to connect to database and perform migrations: {why}");
            panic!("Database connection failed");
        }
    };

    // Set up bot
    let framework: Framework<spoticord_session::manager::SessionManager, anyhow::Error> =
        Framework::builder()
            .setup(
                |ctx: &serenity::prelude::Context,
                 ready: &serenity::all::Ready,
                 framework: &Framework<
                    spoticord_session::manager::SessionManager,
                    anyhow::Error,
                >| Box::pin(bot::setup(ctx, ready, framework, database)),
            )
            .options(bot::framework_opts())
            .build();

    let mut client = match ClientBuilder::new(
        spoticord_config::discord_token(),
        spoticord_config::discord_intents(),
    )
    .framework(framework)
    .register_songbird_from_config(songbird::Config::default().use_softclip(false))
    .await
    {
        Ok(client) => client,
        Err(why) => {
            error!("Fatal error when building Serenity client: {why}");
            panic!("Bot init failed");
        }
    };

    if let Err(why) = client.start_autosharded().await {
        error!("Fatal error occured during bot operations: {why}");
        error!("Bot will now shut down!");
        return Err(shuttle_runtime::Error::Custom(anyhow::anyhow!(
            "Bot failed: {why}"
        )));
    }

    Ok(client.into())
}
