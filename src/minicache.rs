use std::{sync::{Arc, RwLock}, collections::{BinaryHeap, HashSet}};

use nohash_hasher::NoHashHasher;
use twilight_model::id::{Id, marker::UserMarker};
type Expiries =  Arc<RwLock<BinaryHeap<Expiry>>>;
#[derive(Debug, Clone)]
pub struct MessagingCache {
    tokens: Arc<RwLock<HashSet<Id<UserMarker>, NoHashHasher<u64>>>>,
    expiries: Expiries
}


impl MessagingCache {
    pub fn new() -> Self {
        let expiries = Arc::new(RwLock::new(BinaryHeap::new()));
        std::thread::spawn(move || Self::refresh_expiries(expiries.clone()) );
        Self {
            tokens: Arc::new(RwLock::new(HashSet::with_hasher(nohash_hasher::NoHashHasher::default()))),
            expiries
        }
    }
    fn refresh_expiries(expiries: Expiries) {
        loop {
            {
                let expiries = expiries.write().unwrap();
                Self::remove_expired(expiries)
            }
        }
    }
    fn remove_expired(expiries: std::sync::RwLockWriteGuard<'_, BinaryHeap<Expiry>>) {
        
        while let Some(val) = expiries.peek() {
            if val.when > std::time::Instant::now() {}
        }
    
    }
}

#[derive(Hash, PartialEq, Eq, Clone, Debug)]
struct Expiry {
    when: std::time::Instant,
    what: Id<UserMarker>
}

impl std::cmp::PartialOrd for Expiry {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.when.partial_cmp(&other.when).map(|v| v.reverse())
    }
}
impl std::cmp::Ord for Expiry {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.when.cmp(&other.when).reverse()
    }
}