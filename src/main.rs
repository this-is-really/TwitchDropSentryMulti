use std::{collections::{BTreeMap, HashMap, HashSet, VecDeque}, error::Error, path::{Path, PathBuf}, sync::Arc, time::Duration};

use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use rand::{rng, seq::{IndexedRandom, SliceRandom}};
use tokio::{fs::{self}, sync::{Notify, broadcast::{self, Receiver, error::TryRecvError}, watch::Sender}, time::{sleep}};
use tracing::{debug, info};
use tracing_appender::rolling;
use tracing_subscriber::fmt::{time::ChronoLocal, writer::BoxMakeWriter};
use twitch_gql_rs::{TwitchClient, client_type::ClientType, error::ClaimDropError, structs::{DropCampaigns}};

mod r#static;
mod stream;
mod config;
mod webhook;
mod web;

use crate::{config::*, r#static::*, stream::*, web::{AppState, start_api}, webhook::{WebhookSendFormat, webhook_message_worker}};

const STREAM_SLEEP: u64 = 59;
const MAX_COUNT: u64 = 3;

async fn create_client (home_dir: &Path, proxies: &[String]) -> Result<(), Box<dyn Error>> {
    let random_proxy = proxies.choose(&mut rng()).cloned();

    let client_type = ClientType::android_app();
    let mut client = TwitchClient::new(&client_type, &random_proxy).await?;
    let mut count = 0;

    loop {
        count += 1;
        if count >= MAX_COUNT {
            tracing::warn!("Authentication failed: maximum retry attempts ({MAX_COUNT}) reached.");
            return Ok(());
        }
        info!("Starting Twitch device authentication (attempt {count}/{MAX_COUNT})");
        let get_auth = client.request_device_auth().await?;
        println!("To authenticate, open the following URL in your browser:\n{}", get_auth.verification_uri);
        match client.auth(get_auth).await {
            Ok(_) => break,
            Err(twitch_gql_rs::error::AuthError::DeviceTokenExpired) => {
                tracing::warn!("Device authentication token expired. Requesting a new one (attempt {count}/{MAX_COUNT})...");
                continue
            },
            Err(twitch_gql_rs::error::AuthError::TwitchError(e)) => {
                tracing::error!("Twitch returned an error during authentication: {e}");
                return Ok(());
            }
        }
    }
    let path = home_dir.join(format!("{}.json", client.login.clone().unwrap()));
    let path = Path::new(&path);
    if !path.exists() {
        client.save_file(&path).await?;
    }
    let client = TwitchClient::load_from_file(&path, &random_proxy).await?;
    let login = client.login.clone().unwrap_or_default();

    let mut accounts = ACCOUNTS.lock().await;
    let already_exists = if let Some(accs) = &*accounts {
        accs.iter().any(|c| c.login.as_ref().map_or(false, |l| l == &login))
    } else {
        false
    };

    if already_exists {
        println!("Account {} has already been added", login);
        return Ok(());
    }

    match & mut *accounts {
        Some(account) => account.push(Arc::new(client.clone())),
        None => *accounts = Some(vec![Arc::new(client.clone())])
    }
    Ok(())
}

#[tokio::main]
async fn main () -> Result<(), Box<dyn Error>> {
    let file_appender = rolling::never(".", "app.log");
    tracing_subscriber::fmt().with_writer(BoxMakeWriter::new(file_appender)).with_ansi(false).with_timer(ChronoLocal::new("%Y-%m-%d %H:%M:%S%.3f".into())).init();
    let home_dir = Path::new("data");
    if !home_dir.exists() {
        fs::create_dir_all(&home_dir).await?;
    }

    let config_path = home_dir.join("config.json");
    if !config_path.exists() {
        let config = Config::new().await?;
        config.save(&config_path).await?
    }

    let config = Config::load(&config_path).await?;
    config.configure_autostart()?;

    let mut proxies = config.load_proxies_list().await?;

    let mut rng = rng();
    proxies.shuffle(&mut rng);

    let mut proxy_pool = proxies.iter().cycle();

    let mut loaded_clients = Vec::new();
    let mut entries = fs::read_dir(&home_dir).await?;
    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();
        if path.is_file() && path.extension().map_or(false, |s| s == "json" ) && path.file_name().unwrap_or_default() != "cash.json" && path.file_name().unwrap_or_default() != "config.json" {
            let selected_proxy = proxy_pool.next();
            let client = TwitchClient::load_from_file(&path, &selected_proxy.cloned()).await?;
            loaded_clients.push(Arc::new(client));
        }
    }

    if !loaded_clients.is_empty() {
        let mut lock = ACCOUNTS.lock().await;
        *lock = Some(loaded_clients);
    }

    let games = config.loaded_games().await?;

    let clients = ACCOUNTS.lock().await;
    let client = if let Some(accounts) = clients.clone() {
        accounts.first().cloned().unwrap()
    } else {
        return Err("Didn't find accounts")?;
    };
    drop(clients);

    let campaign = client.get_campaign().await?;
    let campaign = campaign.dropCampaigns;
    let mut id_to_index = HashMap::new();
    let mut grouped: BTreeMap<usize, VecDeque<DropCampaigns>> = BTreeMap::new();
    let mut next_index: usize = 0;
    for obj in campaign {
        if obj.status == "EXPIRED" {
            continue;
        }
        let idx = *id_to_index.entry(obj.game.id.clone()).or_insert_with(|| {
            let i = next_index;
            next_index += 1;
            i
        });
        grouped.entry(idx).or_default().push_front(obj);
    }

    start_api(AppState { config: config.clone(), home_dir: home_dir.to_path_buf() }).await?;
    main_logic(client, grouped, home_dir, &games, config.discord_webhook_url.clone(), &proxies).await?; 
    Ok(())
}

async fn main_logic (client: Arc<TwitchClient> ,grouped: BTreeMap<usize, VecDeque<DropCampaigns>>, home_dir: &Path, games: &VecDeque<String>, webhook_url: String, proxies: &[String]) -> Result<(), Box<dyn Error>> {
    let current_campaigns: VecDeque<VecDeque<DropCampaigns>> = if !games.is_empty() {
        games.iter().filter_map(|game_name| {
            let campaigns_for_game: VecDeque<_> = grouped.values().flat_map(|campaigns_vec| {
                campaigns_vec.iter().filter(|campaign| campaign.game.displayName.to_lowercase().trim() == game_name.to_lowercase().trim()).cloned()
            }).collect();

            if campaigns_for_game.is_empty() {
                None
            } else {
                Some(campaigns_for_game)
            }
        }).collect()
    } else {
        for (id, obj) in &grouped {
            for i in obj {
                println!("{} | {}", id, i.game.displayName);
            }
        }
        let input: usize = dialoguer::Input::new().with_prompt("Select game").interact_text()?;
        let selected = grouped.get(&input).cloned().unwrap_or_default();
        vec![selected].into()
    };

    if current_campaigns.is_empty() {
        return Err("No campaigns found for the selected game")?;
    }

    let (webhook_tx, webhook_rx) = tokio::sync::mpsc::channel(100);
    let (drop_id_tx, mut drop_id_rx) = tokio::sync::watch::channel(String::new());
    let (channel_tx, channel_rx) = broadcast::channel(100);
    let channel_rx2 = channel_tx.subscribe();

    let notify = Arc::new(Notify::new());
    let drop_campaigns = Arc::new(current_campaigns.clone());
    let drop_cash_dir = home_dir.join("cash.json");

    let clients = ACCOUNTS.lock().await;
    let clients = if let Some(accounts) = clients.clone() {
        accounts
    } else {
        return Err("Didn't find accounts")?;
    };


    let weebhook_is_active = if !webhook_url.is_empty() {
        webhook_message_worker(webhook_url, webhook_rx, proxies).await;
        true
    } else {
        false
    };
    watch_sync(clients.clone(), channel_rx, notify.clone()).await;
    info!("Watch synchronization task has been successfully initiated");
    drop_sync(clients.clone(), drop_id_tx, drop_cash_dir, channel_rx2, notify.clone(), webhook_tx, weebhook_is_active).await;
    info!("Drop progress tracker is active");
    filter_streams(client.clone(), drop_campaigns.clone()).await;
    info!("Stream filtering has begun");
    update_stream(channel_tx, notify).await;
    info!("Stream priority updated");

    let mut pending_drops: HashSet<String> = HashSet::new();
    {
        let cash = DROP_CACHE.lock().await.clone();

        for game_campaign in current_campaigns {
            for campaign in game_campaign {
                let mut campaign_details = retry!(client.get_campaign_details(&campaign.id));
                for (_, claimed_drops) in &cash {
                    for drop_id_cache in claimed_drops {
                        if let Some(pos) = campaign_details.timeBasedDrops.iter().position(|d| d.id == *drop_id_cache) {
                            campaign_details.timeBasedDrops.remove(pos);
                        }
                    }
                }
                for drop in campaign_details.timeBasedDrops {
                    pending_drops.insert(drop.id);
                }
        }
        }
    }

    while !pending_drops.is_empty() {
        drop_id_rx.changed().await.ok();
        let drop_id = drop_id_rx.borrow().clone();
        if !drop_id.is_empty() && pending_drops.remove(&drop_id) {
            info!("Drop {} processed (remaining: {})", drop_id, pending_drops.len());
        }
    }

    info!("✅ All drops for the selected game are claimed!");
    Ok(())
}

async fn watch_sync (clients: Vec<Arc<TwitchClient>>, rx: Receiver<Channel>, notify: Arc<Notify>) {
    for client in clients {
        let mut rx = rx.resubscribe();
        let notify = notify.clone();
        tokio::spawn(async move {
            let mut old_stream_name = String::new();
            let mut now_watching_stream: Option<(String, String, String)> = None;

            let mut watching = rx.recv().await.unwrap();
            loop {
                match rx.try_recv() {
                    Ok(channel) => watching = channel,
                    Err(TryRecvError::Closed) => tracing::error!("Closed"),
                    Err(_) => {}
                };

                if old_stream_name.is_empty() || old_stream_name != watching.channel_login {
                    info!("Now actively watching channel {}", watching.channel_login);
                    old_stream_name = watching.channel_login.clone();
                    now_watching_stream = None;
                }

                let (stream_id, game_name, game_id) = match &now_watching_stream {
                    Some(s) => s.clone(),
                    None => {
                        let stream_info = retry!(client.get_stream_info(&watching.channel_login));

                        if let Some(stream) = stream_info.stream {
                            let data = (stream.id, stream_info.broadcastSettings.game.name, stream_info.broadcastSettings.game.id);
                            now_watching_stream = Some(data.clone());
                            data
                        } else {
                            debug!("Stream is not live: {}", watching.channel_login);
                            notify.notify_one();
                            sleep(Duration::from_secs(STREAM_SLEEP)).await;
                            continue;
                        }
                    }
                };

                match client.send_watch(&watching.channel_login, &stream_id, &watching.channel_id, Some(&game_name), Some(&game_id)).await {
                    Ok(_) => {
                        sleep(Duration::from_secs(STREAM_SLEEP)).await
                    },
                    Err(e) => {
                        tracing::error!("{e}");
                        sleep(Duration::from_secs(STREAM_SLEEP)).await;
                    }
                }
            }
        });
    }
}

async fn drop_sync(clients: Vec<Arc<TwitchClient>>, tx: Sender<String>, cache_path: PathBuf, rx_watch: broadcast::Receiver<Channel>, notify: Arc<Notify>, webhook_tx: tokio::sync::mpsc::Sender<WebhookSendFormat>, webhook_is_active: bool) {
    if !cache_path.exists() {
        retry!(fs::write(&cache_path, "{}"));
    } else {
        let mut cache = DROP_CACHE.lock().await;
        let cache_str = retry!(fs::read_to_string(&cache_path));
        let cache_vec: HashMap<String, HashSet<String>> = serde_json::from_str(&cache_str).unwrap();
        *cache = cache_vec;
        drop(cache);
    }

    let bars = Arc::new(MultiProgress::new());

    for client in clients {
        let mut rx_watch = rx_watch.resubscribe();
        let notify = notify.clone();
        let tx = tx.clone();
        let webhook_tx = webhook_tx.clone();
        let cache_path = cache_path.clone();
        let bars = bars.clone();

        tokio::spawn(async move {
            let mut last_claimed = String::new();
            let mut last_message = String::new();

            //bar
            let bar = bars.add(ProgressBar::new(1));
            bar.set_style(ProgressStyle::with_template("[{bar:40.cyan/blue}] {percent:.1}% ({pos}/{len} min) {msg}").unwrap());
            bar.set_message("Initialization...");
            bar.enable_steady_tick(Duration::from_millis(500));

            let mut watching = rx_watch.recv().await.unwrap();
            let mut last_drop_id = String::new();

            loop {
                match rx_watch.try_recv() {
                    Ok(new_watch) => {
                        watching = new_watch;
                        last_claimed.clear();
                        last_drop_id.clear();
                    }
                    Err(TryRecvError::Closed) => break,
                    Err(_) => {}
                }

                let drop_progress = retry!(client.get_current_drop_progress_on_channel(&watching.channel_login));

                let should_claim = !drop_progress.dropID.is_empty() && drop_progress.currentMinutesWatched >= drop_progress.requiredMinutesWatched && drop_progress.dropID != last_claimed;

                let mut cache = DROP_CACHE.lock().await;

                if should_claim {
                    retry!(claim_drop(&client, &drop_progress.dropID));
                    info!("Drop claimed: {}", drop_progress.dropID);

                    tx.send(drop_progress.dropID.clone()).unwrap_or_else(|_| tracing::error!("tx closed"));

                    cache.entry(client.user_id.clone().unwrap_or_default()).or_default().insert(drop_progress.dropID.clone());

                    last_claimed = drop_progress.dropID.clone();

                    let cache_string = serde_json::to_string_pretty(&*cache).unwrap();
                    retry!(fs::write(&cache_path, cache_string.as_bytes()));
                }

                drop(cache);

                let message = if drop_progress.dropID.is_empty() {
                    "No active drop • waiting..."
                } else if drop_progress.currentMinutesWatched >= drop_progress.requiredMinutesWatched {
                    "✅ Ready to claim!"
                } else {
                    "Watching"
                };

                if webhook_is_active {
                    if last_message != message {
                        let progress_percent = if drop_progress.requiredMinutesWatched > 0 {
                            ((drop_progress.currentMinutesWatched as f64 / drop_progress.requiredMinutesWatched as f64) * 100.0) as u8
                        } else { 0 };

                        let progress_text = format!("{}m / {}m", drop_progress.currentMinutesWatched, drop_progress.requiredMinutesWatched);

                        let (game_name, game_avatar_url) = if drop_progress.dropID.is_empty() {
                            ("None".to_string(), "None".to_string())
                        } else {
                            let inv = retry!(client.get_inventory());
                            if let Some(found) = inv.inventory.dropCampaignsInProgress.as_ref().and_then(|campaigns| {
                                campaigns.iter().find(|campaign| {
                                    campaign.timeBasedDrops.iter().any(|time_based| {
                                        time_based.id == drop_progress.dropID
                                    })
                                })
                            }) {
                                (found.game.name.clone(), found.imageURL.clone())   
                            } else {
                                (drop_progress.game.map(|game| game.displayName).unwrap_or_else(|| "Unknown".to_string()), "None".to_string())
                            }
                        };

                        let payload = WebhookSendFormat {
                            twitch_name: client.login.clone().unwrap_or("undefined".to_string()),
                            game_name,
                            game_avatar_url,
                            streamer_name: watching.channel_login.clone(),
                            progress_percent,
                            progress_text,
                            status: message.to_string()
                        };
                        let _ = webhook_tx.send(payload).await;
                    }
                }

                last_message = message.to_string();

                let message = format!("{} | {}", client.login.clone().unwrap_or_default(), message);
            
                if drop_progress.dropID != last_drop_id {
                    last_drop_id = drop_progress.dropID.clone();
                    bar.set_position(0);
                    bar.set_length(drop_progress.requiredMinutesWatched.max(1));
                    bar.set_message(message);
                } else {
                    bar.set_message(message);
                }
            
                bar.set_length(drop_progress.requiredMinutesWatched.max(1));
                bar.set_position(drop_progress.currentMinutesWatched);
            
                if drop_progress.dropID.is_empty() || (drop_progress.currentMinutesWatched >= drop_progress.requiredMinutesWatched && drop_progress.dropID != last_claimed) {
                    debug!("Not claiming yet: dropID: {}, currentMinutesWatched: {}, requiredMinutesWatched: {}, lastClaimed: {}", drop_progress.dropID, drop_progress.currentMinutesWatched, drop_progress.requiredMinutesWatched, last_claimed);
                    notify.notify_one();
                    if drop_progress.dropID.is_empty() {
                        let mut lock = CHANNEL_IDS.lock().await;
                        lock.retain(|c| c.channel_id != watching.channel_id);
                    }
                }

                sleep(Duration::from_secs(30)).await;
            }
        });
    }
}

async fn claim_drop (client: &Arc<TwitchClient>, drop_progress_id: &str) -> Result<(), Box<dyn Error + Send + Sync>> {
    loop {
        let inv = retry!(client.get_inventory());
        if let Some(campaigns_in_progress) = inv.inventory.dropCampaignsInProgress {
            for in_progress in campaigns_in_progress {
                for time_based in in_progress.timeBasedDrops {
                    if time_based.id == drop_progress_id {
                        if let Some(id) = time_based.self_drop.dropInstanceID {
                            match client.claim_drop(&id).await {
                                Ok(_) => return Ok(()),
                                Err(ClaimDropError::DropAlreadyClaimed) => return Ok(()),
                                Err(e) => tracing::error!("{e}")
                            }
                        }
                    }
                }
            }
        }
    }
}