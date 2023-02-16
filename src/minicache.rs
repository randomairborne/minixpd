use std::{sync::Arc, time::Duration};

use ahash::{AHashMap, AHashSet};
use parking_lot::RwLock;
use twilight_model::id::{
    marker::{GuildMarker, UserMarker, InteractionMarker},
    Id,
};

#[derive(Debug, Clone)]
pub struct TokenCache {
    tokens: Arc<RwLock<AHashMap<Id<InteractionMarker>, String>>>,
}

impl TokenCache {
    pub fn new() -> Self {
        Self::default()
    }
    pub async fn set(&self, interaction: Id<InteractionMarker>, token: String) {
        if self.tokens.write().insert(interaction, token).is_none() {
            let possible_clear = Arc::downgrade(&self.tokens);
            tokio::spawn(async move {
                tokio::time::sleep(Duration::from_secs(60 * 15)).await;
                if let Some(clear) = possible_clear.upgrade() {
                    clear.write().remove(&interaction);
                }
            });
        }
    }
    pub fn get(&self, interaction: Id<InteractionMarker>, token: String) -> bool {
        self.tokens.read().contains_key(&interaction)
    }
}

impl Default for TokenCache {
    fn default() -> Self {
        Self {
            tokens: Arc::new(RwLock::new(AHashMap::new())),
        }
    }
}

pub type IdSet = (Id<GuildMarker>, Id<UserMarker>);

#[derive(Debug, Clone)]
pub struct MessagingCache {
    users: Arc<RwLock<AHashSet<IdSet>>>,
}

impl MessagingCache {
    pub fn new() -> Self {
        Self::default()
    }
    pub async fn add(&self, guild: Id<GuildMarker>, user: Id<UserMarker>) {
        if self.users.write().insert((guild, user)) {
            let possible_clear = Arc::downgrade(&self.users);
            tokio::spawn(async move {
                tokio::time::sleep(Duration::from_secs(60)).await;
                if let Some(clear) = possible_clear.upgrade() {
                    clear.write().remove(&(guild, user));
                }
            });
        }
    }
    pub fn contains(&self, guild: Id<GuildMarker>, user: Id<UserMarker>) -> bool {
        self.users.read().contains(&(guild, user))
    }
}

impl Default for MessagingCache {
    fn default() -> Self {
        Self {
            users: Arc::new(RwLock::new(AHashSet::new())),
        }
    }
}