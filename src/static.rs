use std::{collections::{HashMap, HashSet}, sync::Arc, time::Duration};

use tokio::{sync::Mutex, time::sleep};
use twitch_gql_rs::{TwitchClient, structs::{Channels, GameDirectory}};

#[derive(Default, Debug, Clone, PartialEq, Eq, Hash)]
pub struct Channel {
    pub channel_id: String,
    pub channel_login: String
}

const MAX_ATTEMPTS: u32 = 3;

#[derive(Debug, Default)]
pub struct AppState {
    pub accounts: Mutex<Option<Vec<Arc<TwitchClient>>>>,
    pub drop_cache: Mutex<HashMap<String, HashSet<String>>>,
    pub channel_ids: Mutex<HashSet<Channel>>,
    pub default_channels: Mutex<HashMap<String, HashSet<GameDirectory>>>,
    pub allow_channeld: Mutex<HashMap<String, HashSet<Channels>>>,
    pub campaign_priority: Mutex<HashMap<String, u32>>
}

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