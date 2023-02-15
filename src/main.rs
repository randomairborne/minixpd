#![deny(clippy::all, clippy::pedantic, clippy::nursery)]

mod cmd_defs;
mod handler;
mod levels;
mod minicache;
mod processor;
mod render_card;
mod toy;

use futures::StreamExt;
use render_card::SvgState;
use sqlx::PgPool;
use std::sync::Arc;
use tracing_subscriber::{prelude::__tracing_subscriber_SubscriberExt, util::SubscriberInitExt};
use twilight_gateway::{
    stream::ShardEventStream, CloseFrame, Config, Event, Intents, MessageSender, Shard,
};
use twilight_model::id::{marker::ApplicationMarker, Id};

#[macro_use]
extern crate tracing;
#[macro_use]
extern crate sqlx;

const THEME_COLOR: u32 = 0x33_33_66;

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(tracing_subscriber::EnvFilter::from_env("LOG"))
        .init();
    let token =
        std::env::var("DISCORD_TOKEN").expect("Expected environment variable DISCORD_TOKEN");
    let database_url =
        std::env::var("DATABASE_URL").expect("Expected environment variable DATABASE_URL");
    println!("Connecting to database {database_url}");
    let db = PgPool::connect(&database_url)
        .await
        .expect("Failed to connect to the database!");
    sqlx::migrate!()
        .run(&db)
        .await
        .expect("Failed to run database migrations!");
    let client = Arc::new(twilight_http::Client::new(token));
    println!("Creating commands...");
    let my_id = client
        .current_user_application()
        .await
        .expect("Failed to get own app ID!")
        .model()
        .await
        .expect("Failed to convert own app ID!")
        .id;
    cmd_defs::register(client.interaction(my_id)).await;
    let http = reqwest::Client::new();
    let svg = SvgState::new();

    let client = twilight_http::Client::new(token.clone());
    let intents = Intents::GUILD_MESSAGES;
    let config = Config::new(token, intents);

    let mut shards: Vec<Shard> =
        twilight_gateway::stream::create_recommended(&client, config, |_, builder| builder.build())
            .await
            .expect("Failed to create reccomended shard count")
            .collect();
    let shard_closers: Vec<MessageSender> = shards.iter().map(Shard::sender).collect();
    tokio::spawn(async move {
        tokio::signal::ctrl_c()
            .await
            .expect("Failed to listen to ctrl-c");
        println!("Shutting down...");
        for shard in shard_closers {
            shard.close(CloseFrame::NORMAL).ok();
        }
    });
    let mut events = ShardEventStream::new(shards.iter_mut());
    println!("Connecting to discord");
    let client = Arc::new(client);
    while let Some((_shard, event_result)) = events.next().await {
        match event_result {
            Ok(event) => {
                let redis = match redis.get().await {
                    Ok(v) => v,
                    Err(e) => {
                        eprintln!("ERROR: Fatal redis error: {e}");
                        return;
                    }
                };
                let client = client.clone();
                let db = db.clone();
                tokio::spawn(async move {
                    if let Err(e) = handle_event(event, db, redis, client).await {
                        eprintln!("Handler error: {e}");
                    }
                });
            }
            Err(e) => eprintln!("Shard loop error: {e}"),
        }
    }
    println!("Done, see ya!");
}

async fn handle_event(
    event: Event,
    db: PgPool,
    redis: deadpool_redis::Connection,
    http: Arc<twilight_http::Client>,
) -> Result<(), Error> {
    match event {
        Event::MessageCreate(msg) => message::save(*msg, db, redis, http).await,
        _ => Ok(()),
    }
}

#[derive(Clone)]
pub struct AppState {
    pub db: PgPool,
    pub pubkey: Arc<String>,
    pub client: Arc<twilight_http::Client>,
    pub my_id: Id<ApplicationMarker>,
    pub svg: SvgState,
    pub http: reqwest::Client,
}
