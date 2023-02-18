use std::{sync::Arc, time::Duration};

use ahash::AHashSet;
use parking_lot::RwLock;
use twilight_model::id::{
    marker::{GuildMarker, UserMarker},
    Id,
};

pub type IdSet = (Id<GuildMarker>, Id<UserMarker>);

#[derive(Debug, Clone)]
pub struct MessagingCache {
    users: Arc<RwLock<AHashSet<IdSet>>>,
}

// This API should be broken out, but that's a lot of work and this is designed for single instances anyway.
impl MessagingCache {
    pub fn new() -> Self {
        Self::default()
    }
    /// Adds an item to the cache, with a 60-second TTL.
    pub async fn add(&self, guild: Id<GuildMarker>, user: Id<UserMarker>) {
        // insert returns true if the value is new- that is, if it hasn't been in there yet.
        // We don't want tasks to remove it if it already exists, because we assume one
        // has already been spawned.
        if self.users.write().insert((guild, user)) {
            // the weak pointer is a workaround for tokio tasks having to be static.
            // It would be fine to make this static, but the struct is more versatile this way.
            let possible_clear = Arc::downgrade(&self.users);
            // tokio tasks are incredibly cheap, so this was the best way to do it.
            tokio::spawn(async move {
                tokio::time::sleep(Duration::from_secs(60)).await;
                if let Some(clear) = possible_clear.upgrade() {
                    clear.write().remove(&(guild, user));
                }
            });
        }
    }
    /// Does this [`MessagingCache`] contain an ID-user pair? With this revolutionary function, you can find out!
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
