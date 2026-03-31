use rand::{rng, seq::IndexedRandom};
use reqwest::{Client, Proxy};
use serde_json::json;
use tokio::sync::mpsc::Receiver;

pub struct WebhookSendFormat {
    pub twitch_name: String,
    pub game_name: String,
    pub game_avatar_url: String,
    pub streamer_name: String,
    pub progress_percent: u8,
    pub progress_text: String,
    pub status: String,
}

pub async fn webhook_message_sync(wh_url: String, mut info_rx: Receiver<WebhookSendFormat>, proxies: &Vec<String>) {
    let random_proxy = proxies.choose(&mut rng());

    let client = if let Some(proxy_str) = random_proxy {
        match Proxy::all(proxy_str) {
            Ok(p) => {
                Client::builder().proxy(p).build().unwrap_or_else(|e| {
                    tracing::error!("Failed to build client with proxy: {}", e);
                    Client::new()
                })
            },
            Err(e) => {
                tracing::error!("Proxy error {}: {}", proxy_str, e);
                Client::new()
            }
        }
    } else {
        Client::new()
    };

    tokio::spawn(async move {
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

            if let Err(e) = client.post(&wh_url).json(&payload).send().await {
                tracing::error!("{e}")
            };
        }
    });
}