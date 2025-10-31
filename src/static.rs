use std::{collections::{HashMap, HashSet}, sync::Arc, time::Duration};

use once_cell::sync::Lazy;
use tokio::{sync::Mutex, time::sleep};
use twitch_gql_rs::structs::{Channels, GameDirectory};

#[derive(Default, Clone, PartialEq, Eq)]
pub struct NowWatched {
    pub channel_login: String,
    pub channel_id: String,
    pub stream_id: String,
}

#[derive(Default, Debug, Clone, PartialEq, Eq, Hash)]
pub struct Channel {
    pub channel_id: String,
    pub channel_login: String
}

const MAX_ATTEMPTS: u32 = 3;

pub static DROP_CASH: Lazy<Arc<Mutex<HashSet<String>>>> = Lazy::new(|| Arc::new(Mutex::new(HashSet::new())));

pub static CHANNEL_IDS: Lazy<Arc<Mutex<HashSet<Channel>>>> = Lazy::new(|| Arc::new(Mutex::new(HashSet::new())));

pub static DEFAULT_CHANNELS: Lazy<Arc<Mutex<HashMap<String, HashSet<GameDirectory>>>>> = Lazy::new(|| Arc::new(Mutex::new(HashMap::new())));

pub static ALLOW_CHANNELS: Lazy<Arc<Mutex<HashMap<String, HashSet<Channels>>>>> = Lazy::new(|| Arc::new(Mutex::new(HashMap::new())));

#[macro_export]
macro_rules! retry {
    ($func:expr) => {
        retry_backup(|| $func).await.expect("Retry failed after MAX_ATTEMPTS attempts")
    };
}

pub async fn retry_backup<F, Fut, T, E> (mut f: F) -> Result<T, E> where F: FnMut() -> Fut, Fut: Future<Output = Result<T, E>> {
    let mut attempts = 0;
    loop {
        match f().await {
            Ok(t) => return Ok(t),
            Err(e) => {
                attempts += 1;
                if attempts > MAX_ATTEMPTS {
                    return Err(e);
                } else {
                    sleep(Duration::from_secs(5)).await
                }
            }
        }
    }
}