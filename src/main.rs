use std::{collections::{BTreeMap, HashMap, HashSet}, env, error::Error, path::{Path, PathBuf}, sync::Arc, time::Duration};

use auto_launch::AutoLaunchBuilder;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use serde::{Deserialize, Serialize};
use tokio::{fs::{self}, sync::{Notify, broadcast::{self, Receiver, error::TryRecvError}, watch::Sender}, time::{sleep}};
use tracing::{info};
use tracing_appender::rolling;
use tracing_subscriber::fmt::writer::BoxMakeWriter;
use twitch_gql_rs::{TwitchClient, client_type::ClientType, error::ClaimDropError, structs::DropCampaigns};

use crate::{r#static::{ACCOUNTS, Channel, DROP_CASH, retry_backup}, stream::{filter_streams, update_stream}};
mod r#static;
mod stream;

const STREAM_SLEEP: u64 = 20;
const MAX_COUNT: u64 = 3;

async fn create_client (home_dir: &Path) -> Result<(), Box<dyn Error>> {
    let client_type = ClientType::android_app();
    let mut client = TwitchClient::new(&client_type).await?;
    let mut count = 0;
    loop {
        count += 1;
        if count >= MAX_COUNT {
            tracing::warn!("Authentication failed: maximum retry attempts ({MAX_COUNT}) reached.");
            break;
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
                break;
            }
        }
    }
    let path = home_dir.join(format!("{}.json", client.login.clone().unwrap()));
    let path = Path::new(&path);
    if !path.exists() {
        client.save_file(&path).await?;
    }
    let client = TwitchClient::load_from_file(&path).await?;
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

#[derive(Debug, Serialize, Deserialize)]
struct Settings {
    game: String,
    autostart: bool
}

#[tokio::main]
async fn main () -> Result<(), Box<dyn Error>> {
    let file_appender = rolling::never(".", "app.log");
    tracing_subscriber::fmt().with_writer(BoxMakeWriter::new(file_appender)).with_ansi(false).init();
    let home_dir = Path::new("data");
    if !home_dir.exists() {
        fs::create_dir_all(&home_dir).await?;
    }

    let mut loaded_clients = Vec::new();
    let mut entries = fs::read_dir(&home_dir).await?;
    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();
        if path.is_file() && path.extension().map_or(false, |s| s == "json" ) && path.file_name().unwrap_or_default() != "cash.json" && path.file_name().unwrap_or_default() != "settings.json" {
            let client = TwitchClient::load_from_file(&path).await?;
            loaded_clients.push(Arc::new(client));
        }
    }

    if !loaded_clients.is_empty() {
        let mut lock = ACCOUNTS.lock().await;
        *lock = Some(loaded_clients);
    }

    let settings_path = home_dir.join("settings.json");
    if !settings_path.exists() {
        let settings = serde_json::to_string_pretty(&Settings { game: String::new(), autostart: false })?;
        fs::write(&settings_path, settings.as_bytes()).await?;
    }

    let settings: Settings = {
        let content = fs::read_to_string(&settings_path).await?;
        serde_json::from_str(&content)?
    };

    configure_autostart(&settings)?;

    let items = vec!["Add account", "Start farming"];
    loop {
        let select = if !settings.game.is_empty() {
            1
        } else {
            dialoguer::Select::new().with_prompt("Select option").items(&items).default(0).interact()?
        };

        match select {
            0 => {
                create_client(&home_dir).await.unwrap()
            },
            1 => {
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
                let mut grouped: BTreeMap<usize, Vec<DropCampaigns>> = BTreeMap::new();
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

                    grouped.entry(idx).or_default().push(obj);
                }

                main_logic(client, grouped, home_dir, &settings).await?;
            },
            _ => {}
        } 
    }
}

fn configure_autostart (settings: &Settings) -> Result<(), Box<dyn Error>> {
    let app_path = {
        let path = env::current_exe()?;
        path.to_str().ok_or("Unable to convert executable path to string")?.to_string()
    };
    let auto =AutoLaunchBuilder::new()
        .set_app_name("TwitchDropSentry")
        .set_app_path(&app_path)
        .set_macos_launch_mode(auto_launch::MacOSLaunchMode::LaunchAgent)
        .set_linux_launch_mode(auto_launch::LinuxLaunchMode::XdgAutostart)
        .set_windows_enable_mode(auto_launch::WindowsEnableMode::Dynamic)
        .build()?;

    if settings.autostart {
        if !auto.is_enabled()? {
            auto.enable()?;
        }
    } else {
        auto.disable()?;
    }
    Ok(())
}

async fn main_logic (client: Arc<TwitchClient> ,grouped: BTreeMap<usize, Vec<DropCampaigns>>, home_dir: &Path, settings: &Settings) -> Result<(), Box<dyn Error>> {
    let current_campaigns: Vec<DropCampaigns> = if !settings.game.is_empty() {
        grouped.values().flat_map(|campaign| {
            campaign.iter().filter(|c| c.game.displayName.to_lowercase() == settings.game.to_lowercase()).cloned()
        }).collect()
    } else {
        for (id, obj) in &grouped {
            for i in obj {
                println!("{} | {}", id, i.game.displayName);
            }
        }
        let input: usize = dialoguer::Input::new().with_prompt("Select game").interact_text()?;
        grouped.get(&input).cloned().unwrap_or_default()
    };

    if current_campaigns.is_empty() {
        return Err("No campaigns found for the selected game")?;
    }

    let (tx_watch, mut rx_watch) = tokio::sync::watch::channel(String::new());
    let drop_campaigns = Arc::new(current_campaigns.clone());
    
    let drop_cash_dir = home_dir.join("cash.json");

    let (tx, rx1) = broadcast::channel(100);
    let rx2 = tx.subscribe();
    let notify = Arc::new(Notify::new());

    let clients = ACCOUNTS.lock().await;
    let clients = if let Some(accounts) = clients.clone() {
        accounts
    } else {
        return Err("Didn't find accounts")?;
    };

    watch_sync(clients.clone(), rx1, notify.clone()).await;
    info!("Watch synchronization task has been successfully initiated");
    drop_sync(clients.clone(), tx_watch, drop_cash_dir, rx2, notify.clone()).await;
    info!("Drop progress tracker is active");
    filter_streams(client.clone(), drop_campaigns.clone()).await;
    info!("Stream filtering has begun");
    update_stream(tx, notify).await;
    info!("Stream priority updated");

    let mut pending_drops: HashSet<String> = HashSet::new();
    {
        let cash = DROP_CASH.lock().await.clone();
        for campaign in &current_campaigns {
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
    info!("Starting farming {} campaigns / {} total drops for game '{}'", &current_campaigns.len(), pending_drops.len(), current_campaigns[0].game.displayName);
    
    while !pending_drops.is_empty() {
        rx_watch.changed().await.ok();
        let drop_id = rx_watch.borrow().clone();
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
            let mut stream_id = String::new();

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
                    stream_id.clear();
                }

                if stream_id.is_empty() {
                    let stream = retry!(client.get_stream_info(&watching.channel_login));
                    if let Some(id) = stream.stream {
                        stream_id = id.id
                    } else {
                        notify.notify_one();
                        sleep(Duration::from_secs(STREAM_SLEEP)).await;
                        continue;
                    }
                }

                match client.send_watch(&watching.channel_login, &stream_id, &watching.channel_id).await {
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

async fn drop_sync(clients: Vec<Arc<TwitchClient>>, tx: Sender<String>, cash_path: PathBuf, rx_watch: broadcast::Receiver<Channel>, notify: Arc<Notify>) {
    if !cash_path.exists() {
        retry!(fs::write(&cash_path, "{}"));
    } else {
        let mut cash = DROP_CASH.lock().await;
        let cash_str = retry!(fs::read_to_string(&cash_path));
        let cash_vec: HashMap<String, HashSet<String>> = serde_json::from_str(&cash_str).unwrap();
        *cash = cash_vec;
        drop(cash);
    }

    let bars = Arc::new(MultiProgress::new());

    for client in clients {
        let mut rx_watch = rx_watch.resubscribe();
        let notify = notify.clone();
        let tx = tx.clone();
        let cash_path = cash_path.clone();
        let bars = bars.clone();

        tokio::spawn(async move {
            let mut last_claimed = String::new();

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

                let mut cash = DROP_CASH.lock().await;
                let drop_progress = retry!(client.get_current_drop_progress_on_channel(&watching.channel_login, &watching.channel_id));

                let should_claim = !drop_progress.dropID.is_empty() && drop_progress.currentMinutesWatched >= drop_progress.requiredMinutesWatched && drop_progress.dropID != last_claimed;

                if should_claim {
                    retry!(claim_drop(&client, &drop_progress.dropID));
                    info!("Drop claimed: {}", drop_progress.dropID);

                    tx.send(drop_progress.dropID.clone()).unwrap_or_else(|_| tracing::error!("tx closed"));

                    cash.entry(client.user_id.clone().unwrap_or_default()).or_default().insert(drop_progress.dropID.clone());

                    last_claimed = drop_progress.dropID.clone();

                    let cash_string = serde_json::to_string_pretty(&*cash).unwrap();
                    retry!(fs::write(&cash_path, cash_string.as_bytes()));
                }

                drop(cash);

                let message = if drop_progress.dropID.is_empty() {
                    "No active drop • waiting..."
                } else if drop_progress.currentMinutesWatched >= drop_progress.requiredMinutesWatched {
                    "✅ Ready to claim!"
                } else {
                    "Watching"
                };

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
            
                if drop_progress.dropID.is_empty() || drop_progress.currentMinutesWatched >= drop_progress.requiredMinutesWatched {
                    notify.notify_one();
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
                            loop {
                                match client.claim_drop(&id).await {
                                    Ok(_) => return Ok(()),
                                    Err(ClaimDropError::DropAlreadyClaimed) => return Ok(()),
                                    Err(e) => tracing::error!("{e}")
                                }
                                sleep(Duration::from_secs(5)).await
                            }
                        }
                    }
                }
            }
        }
    }
}