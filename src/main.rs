#![deny(clippy::all, clippy::pedantic, clippy::nursery)]

mod cmd_defs;
mod handler;
mod leaderboard;
mod levels;
mod message;
mod minicache;
mod processor;
mod render_card;
mod toy;

use render_card::SvgState;
use sqlx::PgPool;
use std::sync::{atomic::AtomicBool, Arc};
use tokio::task::JoinSet;
use tracing_subscriber::{prelude::__tracing_subscriber_SubscriberExt, util::SubscriberInitExt};
use twilight_gateway::{CloseFrame, Config, Event, Intents, Shard};
use twilight_model::id::{marker::ApplicationMarker, Id};

#[macro_use]
extern crate tracing;
#[macro_use]
extern crate sqlx;

const THEME_COLOR: u32 = 0x33_33_66;

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();
    if std::env::var("LOG").is_err() {
        std::env::set_var("LOG", "minixpd=INFO");
    }
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(tracing_subscriber::EnvFilter::from_env("LOG"))
        .init();
    let token =
        std::env::var("DISCORD_TOKEN").expect("Expected environment variable DISCORD_TOKEN");
    let database_url =
        std::env::var("DATABASE_URL").expect("Expected environment variable DATABASE_URL");
    info!("Connecting to database {database_url}");
    let db = PgPool::connect(&database_url)
        .await
        .expect("Failed to connect to the database!");
    sqlx::migrate!()
        .run(&db)
        .await
        .expect("Failed to run database migrations!");
    let client = Arc::new(twilight_http::Client::new(token.clone()));
    info!("Creating commands...");
    let my_id = client
        .current_user_application()
        .await
        .expect("Failed to get own app ID!")
        .model()
        .await
        .expect("Failed to convert own app ID!")
        .id;
    cmd_defs::register(client.interaction(my_id)).await;
    let svg = SvgState::new();
    let intents = Intents::GUILD_MESSAGES;
    let config = Config::new(token, intents);
    let cooldowns = minicache::MessagingCache::new();
    let shards: Vec<Shard> =
        twilight_gateway::stream::create_recommended(&client, config, |_, builder| builder.build())
            .await
            .expect("Failed to create reccomended shard count")
            .collect();
    info!("Connecting to discord");
    let state = AppState {
        db,
        client,
        my_id,
        cooldowns,
        svg,
    };
    let should_shutdown = Arc::new(AtomicBool::new(false));

    let mut set = JoinSet::new();

    for shard in shards {
        set.spawn(event_loop(shard, should_shutdown.clone(), state.clone()));
    }

    tokio::signal::ctrl_c().await.unwrap();

    info!("Shutting down..");

    // Instruct the shards to shutdown.
    should_shutdown.store(true, std::sync::atomic::Ordering::Relaxed);

    // Await all tasks to complete.
    while set.join_next().await.is_some() {}
    info!("Done, see ya!");
}

async fn event_loop(mut shard: Shard, should_shutdown: Arc<AtomicBool>, state: AppState) {
    loop {
        match shard.next_event().await {
            Ok(event) => {
                let state = state.clone();
                tokio::spawn(async move {
                    if let Err(e) = handle_event(event, state).await {
                        error!("Handler error: {e}");
                    }
                });
            }
            Err(e) => error!("Shard loop error: {e}"),
        }
        if should_shutdown.load(std::sync::atomic::Ordering::Relaxed) {
            // We're shutting down either way, errors don't matter.
            _ = shard.close(CloseFrame::NORMAL).await;
            break;
        }
    }
}

async fn handle_event(event: Event, state: AppState) -> Result<(), Error> {
    match event {
        Event::MessageCreate(msg) => message::save(*msg, state).await,
        Event::InteractionCreate(i) => handler::handle(i.0, state).await,
        _ => Ok(()),
    }
}

#[derive(Clone)]
pub struct AppState {
    pub db: PgPool,
    pub client: Arc<twilight_http::Client>,
    pub my_id: Id<ApplicationMarker>,
    pub cooldowns: minicache::MessagingCache,
    pub svg: SvgState,
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Discord sent a command that is not known!")]
    UnrecognizedCommand,
    #[error("Discord did not send a user object for the command invoker when it was required!")]
    NoInvoker,
    #[error("Discord did not send a user object for the command target when it was required!")]
    NoTarget,
    #[error("Discord did not send part of the Resolved Data!")]
    NoResolvedData,
    #[error("Discord did not send target ID for message!")]
    NoMessageTargetId,
    #[error("Discord sent interaction data for an unsupported interaction type!")]
    WrongInteractionData,
    #[error("Discord did not send any interaction data!")]
    NoInteractionData,
    #[error("Discord did not send a guild ID!")]
    NoGuildId,
    #[error("Discord did not send a parent message for button!")]
    NoParentMessage,
    #[error("Discord sent unknown custom button ID!")]
    InvalidCustomButtonId,
    #[error("Failed to validate message: {0}!")]
    ValidateMessage(#[from] twilight_validate::message::MessageValidationError),
    #[error("Interaction parser encountered an error: {0}!")]
    Parse(#[from] twilight_interactions::error::ParseError),
    #[error("SVG renderer encountered an error: {0}!")]
    ImageGenerator(#[from] crate::render_card::RenderingError),
    #[error("SQLx encountered an error: {0}")]
    Sqlx(#[from] sqlx::Error),
    #[error("Twilight-HTTP encountered an error: {0}")]
    Http(#[from] twilight_http::Error),
}
