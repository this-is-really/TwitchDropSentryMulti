use std::collections::HashMap;

use rand::{rng, seq::IndexedRandom};
use reqwest::{Client, Proxy};
use serde_json::json;
use tracing::error;
use tokio::sync::mpsc::Receiver;

#[derive(Debug, Default, Clone)]
pub struct WebhookSendFormat {
    pub twitch_name: String,
    pub game_name: String,
    pub game_avatar_url: String,
    pub streamer_name: String,
    pub progress_percent: u8,
    pub progress_text: String,
    pub status: String,
}

pub async fn webhook_message_worker(wh_url: String, mut info_rx: Receiver<WebhookSendFormat>, proxies: &[String]) {
    let random_proxy = proxies.choose(&mut rng());

    let client = if let Some(proxy_str) = random_proxy {
        match Proxy::all(proxy_str) {
            Ok(p) => {
                Client::builder().proxy(p).build().unwrap_or_else(|e| {
                    error!("Failed to build client with proxy: {}", e);
                    Client::new()
                })
            },
            Err(e) => {
                error!("Proxy error {}: {}", proxy_str, e);
                Client::new()
            }
        }
    } else {
        Client::new()
    };

    tokio::spawn(async move {
        let mut account_messages: HashMap<String, String> = HashMap::new();
        while let Some(info) = info_rx.recv().await {
            let progress_bar = (0..10).map(|i| if i < (info.progress_percent / 10) { "▰" } else { "▱" }).collect::<String>();

            let payload = json!({
                "username": "TwitchDropSentryMulti",
                "avatar_url": "https://assets.twitch.tv/assets/mobile_android-d0b749d8e88afd01abd6.png",
                "embeds": [{
                    "title": "🎮 Twitch Drops Farming Update",
                    "color": 10181046,
                    "timestamp": chrono::Utc::now().to_rfc3339(),
                    "thumbnail": { "url": info.game_avatar_url },
                    "fields": [
                        { 
                            "name": "Account", 
                            "value": format!("`{}`", info.twitch_name), 
                            "inline": true 
                        },
                        { 
                            "name": "Game", 
                            "value": info.game_name, 
                            "inline": true 
                        },
                        { 
                            "name": "Streamer", 
                            "value": format!("`{}`", info.streamer_name), 
                            "inline": true 
                        },
                        { 
                            "name": "Drop Progress", 
                            "value": format!("{} **{}%** • {}", progress_bar, info.progress_percent, info.progress_text), 
                            "inline": false 
                        },
                        { 
                            "name": "Status", 
                            "value": info.status, 
                            "inline": false 
                        }
                    ],
                    "footer": { 
                        "text": "TwitchDropSentryMulti • Live Update" 
                    }
                }]
            });

            if let Some(msg_id) = account_messages.get(&info.twitch_name) {
                let edit_url = format!("{}/messages/{}", wh_url, msg_id);
                if let Err(e) = client.patch(edit_url).json(&payload).send().await {
                    error!("Failed to edit message: {}", e);
                }
            } else {
                let post_url = format!("{}?wait=true", wh_url);
                match client.post(post_url).json(&payload).send().await {
                    Ok(response) => {
                        if response.status().is_success() {
                            if let Ok(json_resp) = response.json::<serde_json::Value>().await {
                                if let Some(id) = json_resp.get("id").and_then(|v| v.as_str()) {
                                    account_messages.insert(info.twitch_name.clone(), id.to_string());
                                } else {
                                    error!("Failed to get message ID from response");
                                }
                            } else {
                                error!("Failed to parse JSON response");
                            }
                        }
                    },
                    Err(e) => error!("Failed to send webhook message: {}", e)
                }
            }
        }
    });
}