use std::{collections::{BinaryHeap, HashSet}, error::Error, sync::Arc, time::Duration};

use tokio::sync::{Mutex, Notify, broadcast::{self, Sender}, watch::Receiver};

use futures_util::{SinkExt, StreamExt};
use serde_json::{Value, json};
use tokio::time::sleep;
use tokio_tungstenite::{connect_async, tungstenite::Message};
use tracing::debug;
use twitch_gql_rs::{TwitchClient, structs::{Channels, DropCampaigns, GameDirectory}};

use crate::{retry, r#static::{ALLOW_CHANNELS, CHANNEL_IDS, Channel, DEFAULT_CHANNELS, retry_backup}};

const UPDATE_TIME: u64 = 15;
const MAX_TOPICS: usize = 50;
const WS_URL: &'static str = "wss://pubsub-edge.twitch.tv/v1";

pub async fn filter_streams (client: Arc<TwitchClient>, campaigns: Arc<Vec<DropCampaigns>>) {
    let mut count = 0;
    let mut video_vec = HashSet::new();
    for campaign in campaigns.iter() {
        let campaign_details = retry!(client.get_campaign_details(&campaign.id));
        if let Some(allow) = campaign_details.allow.channels {
            let mut allow_channels = ALLOW_CHANNELS.lock().await;
            let allow: HashSet<Channels> = allow.into_iter().collect();
            allow_channels.insert(campaign.id.to_string(), allow.clone());
            drop(allow_channels);
            for channel in allow {
                let stream_info = retry!(client.get_stream_info(&channel.name));
                if stream_info.stream.is_some() {
                    if count >= MAX_TOPICS {
                        break;
                    }
                    let avaiable_drops = retry!(client.get_available_drops_for_channel(&channel.id));
                    if avaiable_drops.viewerDropCampaigns.is_some() {
                        video_vec.insert(Channel { channel_id: channel.id, channel_login: channel.name });
                        count += 1
                    }
                }
            }
        } else {
            let game_directory = retry!(client.get_game_directory(&campaign_details.game.slug, 30, true));
            let mut all_default = DEFAULT_CHANNELS.lock().await;
            let game_directory: HashSet<GameDirectory> = game_directory.into_iter().collect();
            all_default.insert(campaign.id.to_string(), game_directory.clone());
            drop(all_default);
            for channel in game_directory {
                let stream_info = retry!(client.get_stream_info(&channel.broadcaster.login));
                if count >= MAX_TOPICS {
                    break;
                }
                if stream_info.stream.is_some() {
                    let available_drops = retry!(client.get_available_drops_for_channel(&channel.broadcaster.id));
                    if available_drops.viewerDropCampaigns.is_some() {
                        video_vec.insert(Channel { channel_id: channel.broadcaster.id, channel_login: channel.broadcaster.login });
                        count += 1
                    }
                }
            }
        }
    }
    let mut lock = CHANNEL_IDS.lock().await;
    *lock = video_vec;
    drop(lock);
    debug!("Drop lock video");
    spawn_ws(client.access_token.clone().unwrap()).await;

    tokio::spawn(async move {
        loop {
            let lock = CHANNEL_IDS.lock().await;
            let count = lock.len();
            drop(lock);
            if count < MAX_TOPICS {
                let mut to_add = HashSet::new();
                for campaign in campaigns.iter() {
                    let allow_channels = ALLOW_CHANNELS.lock().await;
                    if let Some(channels) = allow_channels.get(&campaign.id) {
                        for channel in channels {
                            if to_add.len() + count  >= MAX_TOPICS {
                                break;
                            }
                            let stream_info = retry!(client.get_stream_info(&channel.name));
                            if stream_info.stream.is_some() {
                                let available_drops = retry!(client.get_available_drops_for_channel(&channel.id));
                                if available_drops.viewerDropCampaigns.is_some() {
                                    to_add.insert(Channel { channel_id: channel.id.clone(), channel_login: channel.name.clone() });
                                }
                            }
                        }
                    } else {
                        let mut default_channels = DEFAULT_CHANNELS.lock().await;
                        let slug = retry!(client.get_slug(&campaign.game.displayName));
                        let game_directory = retry!(client.get_game_directory(&slug, 30, true));
                        let game_directory: HashSet<GameDirectory> = game_directory.into_iter().collect();
                        default_channels.insert(campaign.id.clone(), game_directory.clone());
                        for channel in &game_directory {
                            let available_drops = retry!(client.get_available_drops_for_channel(&channel.broadcaster.id));
                            if available_drops.viewerDropCampaigns.is_some() {
                                to_add.insert(Channel { channel_id: channel.broadcaster.id.clone(), channel_login: channel.broadcaster.login.clone() });
                                if to_add.len() + count  >= MAX_TOPICS {
                                    break;
                                }
                            }
                        }
                        drop(default_channels);
                    }
                    if to_add.len() + count >= MAX_TOPICS {
                        break;
                    }

                }

                let mut lock = CHANNEL_IDS.lock().await;
                let mut cur = lock.len();
                for channel in to_add {
                    if cur >= MAX_TOPICS { 
                        break 
                    };
                    lock.insert(channel);
                    cur += 1;
                }
                drop(lock);
            }
            debug!("Drop ids");
            sleep(Duration::from_secs(UPDATE_TIME)).await
        }
    });
}

//ws_logick
async fn spawn_ws (auth_token: String) {
    tokio::spawn(async move {
        loop {
            let mut send_channels: HashSet<Channel> = HashSet::new();
            let (ws_stream, _) = retry!(connect_async(WS_URL));
            let (mut write, mut read) = ws_stream.split();
            loop {
                let mut channel_ids = CHANNEL_IDS.lock().await;
                let new_channels: Vec<Channel> = channel_ids.iter().filter(|id| !send_channels.contains(*id)).cloned().collect();
                let delete_channels: Vec<Channel> = send_channels.iter().filter(|id| !channel_ids.contains(&id)).cloned().collect();

                if !new_channels.is_empty() {
                    let topics: Vec<String> = new_channels.iter().map(|channel| format!("video-playback-by-id.{}", channel.channel_id)).collect();
                    let payload = json!({
                        "type": "LISTEN",
                        "data": {
                            "topics": topics,
                            "auth_token": auth_token
                        }
                    });
                    let payload = serde_json::to_string(&payload).unwrap();
                    let payload = tokio_tungstenite::tungstenite::Message::Text(payload.into());
                    write.send(payload).await.unwrap_or_else(|e| tracing::error!("Failed to send payload to WebSocket: {e}"));
                    send_channels.extend(new_channels);
                }

                if !delete_channels.is_empty() {
                    let delete_topics: Vec<String> = delete_channels.iter().map(|channel| format!("video-playback-by-id.{}", channel.channel_id)).collect();
                    let payload = json!({
                        "type": "UNLISTEN",
                        "data": {
                            "topics": delete_topics,
                            "auth_token": auth_token
                        }
                    });
                    let payload = serde_json::to_string(&payload).unwrap();
                    let payload = tokio_tungstenite::tungstenite::Message::Text(payload.into());
                    write.send(payload).await.unwrap_or_else(|e| tracing::error!("Failed to send payload to WebSocket: {e}"));
                    for delete in delete_channels {
                        send_channels.remove(&delete);
                    }
                };

                if let Some(msg) = read.next().await {
                    match msg {
                        Ok(Message::Text(text)) => {
                            if text.contains("\"type\":\"PING\"") {
                                let pong = Message::Text("{\"type\":\"PONG\"}".into());
                                write.send(pong).await.unwrap();
                            }
                            let json: Value = serde_json::from_str(&text).unwrap();
                            if let Some(err) = json.get("error") {
                                if err != "" {
                                    tracing::error!("{err}")
                                }
                            } else {
                                let data = check_json(&json, "data").unwrap_or_else(|e| {tracing::error!("{e}"); &Value::Null});
                                let message = check_json(&data, "message").unwrap_or_else(|e| {tracing::error!("{e}"); &Value::Null}).as_str().unwrap_or_default();
                                let topic = check_json(data, "topic").unwrap_or_else(|e| { tracing::error!("{e}"); &Value::Null }).as_str().unwrap_or_default();
                                let message_json: Value = serde_json::from_str(&message).unwrap();
                                if let Some(viewers) = message_json.get("viewers").and_then(|s| s.as_u64()) {
                                    if viewers == 0 {
                                        if let Some(id_str) = topic.split('.').last() {
                                            let channel_id_to_remove = channel_ids.iter().find(|channel| channel.channel_id == id_str).cloned();
                                            if let Some(to_remove) = channel_id_to_remove {
                                                channel_ids.remove(&to_remove);
                                            }
                                            send_channels.retain(|channel| channel.channel_id != id_str );
                                        }
                                    }
                                } else {
                                    if let Some(id_str) = topic.split('.').last() {
                                        let channel_id_to_remove = channel_ids.iter().find(|channel| channel.channel_id == id_str).cloned();
                                        if let Some(to_remove) = channel_id_to_remove {
                                            channel_ids.remove(&to_remove);
                                        }
                                        send_channels.retain(|channel| channel.channel_id != id_str );
                                    }
                                }
                            }

                        },
                        Ok(Message::Ping(ping)) => write.send(Message::Pong(ping)).await.unwrap_or_else(|e| tracing::error!("Failed to send PONG to WebSocket: {e}")),
                        Ok(_) => {},
                        Err(_) => {
                            sleep(Duration::from_secs(UPDATE_TIME)).await;
                            break ;
                        } 
                    }
                }
            }
        }
        
        
    });
}

fn check_json<'a>(v: &'a Value, data: &str) -> Result<&'a Value, Box<dyn Error>> {
    if let Some(key) = v.get(&data) {
        return Ok(key);
    } else {
        return Err(format!("Failed to find '{}' in JSON", data))?;
    }
}

#[derive(PartialEq, Eq, Clone)]
struct Priority {
    priority: u32,
    name: Channel
}

impl Ord for Priority {
    fn cmp (&self, other: &Self) -> std::cmp::Ordering {
        other.priority.cmp(&other.priority)
    }
}

impl PartialOrd for Priority {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

async fn send_now_watched (mut rx: Receiver<BinaryHeap<Priority>>, tx_now_watch: broadcast::Sender<Channel>, notify: Arc<Notify>, tx_for_delete: tokio::sync::watch::Sender<Channel>) {
    tokio::spawn(async move {
        loop {
            if let Ok(_) = rx.changed().await {
                let watch = rx.borrow().clone();
                    if let Some(max) = watch.peek() {
                        debug!("Send: {}", max.name.channel_login);
                        if let Err(e) = tx_now_watch.send(Channel { channel_id: max.name.channel_id.to_string(), channel_login: max.name.channel_login.to_string() }) {
                            tracing::error!("{e}")
                        };
                        notify.notified().await;

                        loop {
                            if let Err(e) = tx_for_delete.send(max.name.clone()) {
                                tracing::error!("{e}");
                                sleep(Duration::from_secs(5)).await;
                                continue;
                            } else {
                                break;
                            }
                        }

                        sleep(Duration::from_secs(5)).await;
                        continue;
                    } else {
                        sleep(Duration::from_secs(5)).await;
                        continue;
                    }  
                
            }
            

        }

    });
}

pub async fn update_stream (campaigns: Arc<Vec<DropCampaigns>>, tx_now_watch: Sender<Channel>, notify: Arc<Notify>) {
    tokio::spawn(async move {
        let mut heap: BinaryHeap<Priority> = BinaryHeap::new();
        let mut old_channel_ids: HashSet<Channel> = HashSet::new();
        let (tx, rx) = tokio::sync::watch::channel(BinaryHeap::new());
        let (tx_for_delete, mut rx_for_delete) = tokio::sync::watch::channel(Channel::default());

        send_now_watched(rx, tx_now_watch, notify, tx_for_delete).await;

        let channel_to_delete = Arc::new(Mutex::new(Channel::default()));
        let channel_to_delete_clone = Arc::clone(&channel_to_delete);
        tokio::spawn(async move {
            loop {
                if let Ok(_) = rx_for_delete.changed().await {
                    let channel = rx_for_delete.borrow().clone();
                    let mut lock = channel_to_delete.lock().await;
                    *lock = channel
                }
            }
        });

        loop {
            let channel_ids = CHANNEL_IDS.lock().await.clone();
            let allow_channels = ALLOW_CHANNELS.lock().await.clone();
            let default_channels = DEFAULT_CHANNELS.lock().await.clone();

            if channel_ids.is_empty() || (allow_channels.is_empty() && default_channels.is_empty()) {
                sleep(Duration::from_secs(UPDATE_TIME)).await;
                continue;
            }

            let added_channels: Vec<&Channel> = channel_ids.iter().filter(|id| !old_channel_ids.contains(id)).collect();
            let mut delete_channels: Vec<&Channel> = old_channel_ids.iter().filter(|id| !channel_ids.contains(id)).collect();

            let add_channel_to_delete = channel_to_delete_clone.lock().await;
            if *add_channel_to_delete != Channel::default() {
                delete_channels.push(&add_channel_to_delete);
            }

            if !delete_channels.is_empty() {
                let delete_set: HashSet<&Channel> = delete_channels.into_iter().collect();
                let mut new_heap = BinaryHeap::new();
                while let Some(item) = heap.pop() {
                    if !delete_set.iter().any(|channel| channel.channel_id == item.name.channel_id) {
                        new_heap.push(item);
                    }
                }
                heap = new_heap
            }
            if !added_channels.is_empty() {
                for drop_id in campaigns.iter() {
                    for channel in &channel_ids {
                        debug!("{}", channel.channel_id);
                        debug!("{}", drop_id.id);
                        if let Some(allow) = allow_channels.get(&drop_id.id) {
                                if let Some(channel_allow) = allow.iter().find(|s| s.id == *channel.channel_id) {
                                    let channel = Channel { channel_id: channel_allow.id.clone(), channel_login: channel_allow.name.clone() };
                                    if !channel_ids.contains(&channel) {
                                        heap.push(Priority { priority: 1, name: channel.clone() });
                                    }
                                    debug!("Allow {}", channel_allow.name);
                                    heap.push(Priority { priority: 3, name: channel });
                                }
                        }

                        if let Some(default) = default_channels.get(&drop_id.id) {
                            if let Some(channel_default) = default.iter().find(|s| s.broadcaster.id == *channel.channel_id) {
                                debug!("Default {}", channel_default.broadcaster.login);
                                let channel = Channel { channel_id: channel_default.broadcaster.id.clone(), channel_login: channel_default.broadcaster.login.clone() };
                                if !channel_ids.contains(&channel) {
                                    heap.push(Priority { priority: 0, name: channel.clone() });
                                }
                                heap.push(Priority { priority: 2, name: channel });
                            }
                        }

                    } 
                }
                old_channel_ids = channel_ids
            }
            tx.send(heap.clone()).unwrap();
            sleep(Duration::from_secs(UPDATE_TIME)).await;
        }
    });
}