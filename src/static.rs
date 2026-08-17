use std::{collections::{HashMap, HashSet}, path::PathBuf, sync::Arc, time::Duration};

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
    pub channel_pool: Mutex<HashSet<Channel>>,
    pub default_channels: Mutex<HashMap<String, HashSet<GameDirectory>>>,
    pub allow_channels: Mutex<HashMap<String, HashSet<Channels>>>,
    pub campaign_priority: Mutex<HashMap<String, u32>>,
    pub cache_path: std::sync::OnceLock<PathBuf>,
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

pub fn spawn_supervised<F, Fut>(name: impl Into<String>, restart_delay: Duration, mut make_future: F) 
where 
    F: FnMut() -> Fut + Send + 'static,
    Fut: Future<Output = ()> + Send + 'static
{
    let name = name.into();
    tokio::spawn(async move {
        loop {
            let handle = tokio::spawn(make_future());
            match handle.await {
                Ok(()) => {
                    tracing::warn!("Task '{name}' exited unexpectedly (it should run forever). Restarting in {}s...", restart_delay.as_secs());
                }
                Err(join_err) if join_err.is_panic() => {
                    tracing::error!("Task '{name}' panicked: {join_err}. Restarting in {}s...", restart_delay.as_secs());
                }
                Err(join_err) => {
                    tracing::warn!("Task '{name}' was cancelled: {join_err}");
                    return;
                }
            }
            sleep(restart_delay).await;
        }
    });
} 