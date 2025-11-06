use std::{collections::{BTreeMap, HashMap, HashSet}, error::Error, path::{Path, PathBuf}, sync::Arc, time::Duration};

use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use tokio::{fs::{self}, sync::{Notify, broadcast::{self, Receiver, error::TryRecvError}, watch::Sender}, time::{Instant, sleep}};
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
    let get_auth = client.request_device_auth().await?;
    println!("{}", get_auth.verification_uri);
    client.auth(get_auth).await?;
    let path = home_dir.join(format!("{}.json", client.login.clone().unwrap()));
    let path = Path::new(&path);
    if !path.exists() {
        client.save_file(&path).await?;
    }
    let client = TwitchClient::load_from_file(&path).await?;
    let mut accounts = ACCOUNTS.lock().await;
    match & mut *accounts {
        Some(account) => account.push(Arc::new(client.clone())),
        None => *accounts = Some(vec![Arc::new(client.clone())])
    }
    Ok(())
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
        if path.is_file() && path.extension().map_or(false, |s| s == "json" ) && path.file_name().unwrap_or_default() != "cash.json" {
            let client = TwitchClient::load_from_file(&path).await?;
            loaded_clients.push(Arc::new(client));
        }
    }

    if !loaded_clients.is_empty() {
        let mut lock = ACCOUNTS.lock().await;
        *lock = Some(loaded_clients);
    }

    let items = vec!["Add account", "Start farming"];
    loop {
       let select = dialoguer::Select::new().with_prompt("Select option").items(&items).default(0).interact()?;
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

                for (id, obj) in &grouped {
                    for i in obj {
                        println!("{} | {}", id, i.game.displayName);
                    }
                }

                main_logic(client, grouped, home_dir).await?;
            },
            _ => {}
        } 
    }
}

async fn main_logic (client: Arc<TwitchClient> ,grouped: BTreeMap<usize, Vec<DropCampaigns>>, home_dir: &Path) -> Result<(), Box<dyn Error>> {
    let input: usize = dialoguer::Input::new().with_prompt("Select game").interact_text()?;
    if let Some(current_campaigns) = grouped.get(&input) {

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
        update_stream(drop_campaigns, tx, notify).await;
        info!("Stream priority updated");

        for campaign in current_campaigns {

            let mut campaign_details = client.get_campaign_details(&campaign.id).await?;

            let drop_ids_cache = DROP_CASH.lock().await.clone();    
            for drop_id_cache in drop_ids_cache {
                for id in drop_id_cache.1 {
                    let deleted_time_based = campaign_details.timeBasedDrops.iter().filter(|time_based| time_based.id == id).map(|time_based| time_based.id.clone()).collect::<Vec<String>>();
                    for delete in deleted_time_based {
                        if let Some(pos) = campaign_details.timeBasedDrops.iter().position(|time_based| time_based.id == delete) {
                            campaign_details.timeBasedDrops.remove(pos);
                        }
                    }
                }
            }

            loop {
                rx_watch.changed().await.unwrap();
                let drop_id = rx_watch.borrow();
                if drop_id.is_empty() {
                    sleep(Duration::from_secs(10)).await;
                    continue;
                }
                if campaign_details.timeBasedDrops.is_empty() {
                    break;
                }
                if let Some(pos) = campaign_details.timeBasedDrops.iter().position(|time_based| time_based.id == *drop_id) {
                    campaign_details.timeBasedDrops.remove(pos);
                }
            }
            
        }
    }
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

async fn drop_sync (clients: Vec<Arc<TwitchClient>>, tx: Sender<String>, cash_path: PathBuf, rx_watch: broadcast::Receiver<Channel>, notify: Arc<Notify>) {
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
            let mut end_time = Instant::now() + Duration::from_secs(60*60);
            let mut old_drop = String::new();

            //bar
            let bar = bars.add(ProgressBar::new(1));
            bar.set_style(ProgressStyle::with_template("[{bar:40.cyan/blue}] {percent:.1}% ({pos}/{len} min) {msg}").unwrap());
            bar.set_message("Initialization...");
            bar.enable_steady_tick(Duration::from_millis(500));

            let tolerance = Duration::from_secs(5 * 60);

            let mut count = 0;

            let mut watching = rx_watch.recv().await.unwrap();
            loop {
                match rx_watch.try_recv() {
                    Ok(new_watch) => {
                        count = 0;
                        watching = new_watch
                    },
                    Err(TryRecvError::Closed) => break,
                    Err(_) => {}
                }
                let mut cash = DROP_CASH.lock().await;

                let drop_progress = retry!(client.get_current_drop_progress_on_channel(&watching.channel_login, &watching.channel_id));

                let mut need_update = false;

                if let Some(user_id) = &client.user_id {
                    if let Some(user) = cash.get(user_id) {
                        if end_time <= Instant::now() || old_drop != drop_progress.dropID && !user.contains(&drop_progress.dropID) {
                            retry!(claim_drop(&client, &old_drop));
                            info!("Drop claimed: {}", old_drop);
                            tx.send(old_drop.to_string()).unwrap();
                            cash.entry(user_id.clone()).or_insert(HashSet::new()).insert(old_drop.to_string());
                            old_drop = drop_progress.dropID.to_string();
                            need_update = true;

                            let cash_string_writer = serde_json::to_string_pretty(&cash.clone()).unwrap();
                            retry!(fs::write(&cash_path, cash_string_writer.clone()))
                        }
                    }
                    
                }

                if drop_progress.dropID.is_empty() {
                    count += 1;
                    if count >= MAX_COUNT {
                        drop(cash);
                        notify.notify_one();
                        count = 0;
                        continue;
                    } else {
                        drop(cash);
                        sleep(Duration::from_secs(5)).await;
                        continue;
                    }
                }

                drop(cash);

                if old_drop.is_empty() {
                    old_drop = drop_progress.dropID.to_string()
                }

                bar.set_length(drop_progress.requiredMinutesWatched);
                bar.set_message(format!("User: {}", client.login.clone().unwrap()));
                bar.set_position(drop_progress.currentMinutesWatched);

                if end_time <= Instant::now() + tolerance || Instant::now() <= end_time + tolerance && need_update  {
                    let reaming = drop_progress.requiredMinutesWatched.saturating_sub(drop_progress.currentMinutesWatched);
                    end_time = Instant::now() + Duration::from_secs(reaming * 60);
                }

                sleep(Duration::from_secs(30)).await;
            }
       
        });
    }
}

async fn claim_drop (client: &Arc<TwitchClient>, drop_progress_id: &str) -> Result<(), Box<dyn Error + Send + Sync>> {
    loop {
        let inv = retry!(client.get_inventory());
        for in_progress in inv.inventory.dropCampaignsInProgress {
            for time_based in in_progress.timeBasedDrops {
                if time_based.id == drop_progress_id {
                    if let Some(id) = time_based.self_drop.dropInstanceID {
                        match client.claim_drop(&id).await {
                            Ok(_) => return Ok(()),
                            Err(ClaimDropError::DropAlreadyClaimed) => return Ok(()),
                            Err(e) => return Err(e)?
                        }
                    }
                }
            }
        }
    }
}
