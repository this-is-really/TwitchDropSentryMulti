use std::{collections::VecDeque, env, error::Error, path::Path};

use auto_launch::AutoLaunchBuilder;
use serde::{Deserialize, Serialize};
use tokio::{fs::{self, File}, io::{AsyncBufReadExt, BufReader, Lines}};

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    games_path: String,
    autostart: bool,
    proxies_path: String,
}

async fn open_lines (path: &str) -> Result<Lines<BufReader<File>>, Box<dyn Error>> {
    let path = Path::new(&path);

    let file = match fs::File::open(path).await {
        Ok(f) => f,
        Err(e) => {
            tracing::error!("Failed to read file at: '{}'", path.display());
            return Err(e)?;
        }
    };

    let reader = BufReader::new(file).lines();
    Ok(reader)
}

impl Config {
    pub fn configure_autostart (&self) -> Result<(), Box<dyn Error>> {
        let app_path = {
            let path = env::current_exe()?;
            path.to_str().ok_or("Unable to convert executable path to string")?.to_string()
        };
        let auto = AutoLaunchBuilder::new()
            .set_app_name("TwitchDropSentry")
            .set_app_path(&app_path)
            .set_macos_launch_mode(auto_launch::MacOSLaunchMode::LaunchAgent)
            .set_linux_launch_mode(auto_launch::LinuxLaunchMode::XdgAutostart)
            .set_windows_enable_mode(auto_launch::WindowsEnableMode::Dynamic)
            .build()?;

        if self.autostart {
            if !auto.is_enabled()? {
                auto.enable()?;
            }
        } else {
            auto.disable()?;
        }
        Ok(())
    }

    pub async fn new () -> Result<Self, Box<dyn Error>> {
        let lists_path = Path::new("lists");
        if !lists_path.exists() {
            fs::create_dir(&lists_path).await?;
        };

        let games_path = lists_path.join("games.txt");
        if !games_path.exists() {
            fs::write(&games_path, "".as_bytes()).await?;
        }

        let proxies_path = lists_path.join("proxies.txt");
        if !proxies_path.exists() {
            fs::write(&proxies_path, "".as_bytes()).await?;
        }

        Ok(Config { games_path: "./lists/games.txt".to_string(), autostart: false, proxies_path: "./lists/proxies.txt".to_string() })
    }

    pub async fn save (&self, path: &Path) -> Result<(), Box<dyn Error>> {
        let to_write = serde_json::to_string_pretty(&self)?;
        fs::write(&path, to_write).await?;
        Ok(())
    }

    pub async fn load (path: &Path) -> Result<Self, Box<dyn Error>> {
        let config: Config = {
            let read = fs::read_to_string(&path).await?;
            serde_json::from_str(&read)?
        };
        Ok(config)
    }

    pub async fn load_proxies_list (&self) -> Result<Vec<String>, Box<dyn Error>> {
        let mut reader = open_lines(&self.proxies_path).await?;
        
        let mut proxies = Vec::new();

        while let Some(line) = reader.next_line().await? {
            let trimmed = line.trim();
            if !trimmed.is_empty() {
                proxies.push(line);
            }
        }
        Ok(proxies)
    }

    pub async fn loaded_games (&self) -> Result<VecDeque<String>, Box<dyn Error>> {
    let mut reader = open_lines(&self.games_path).await?;

        let mut games = VecDeque::new();

        while let Some(line) = reader.next_line().await? {
            let trimmed = line.trim().trim_start_matches("\u{feff}");
            if !trimmed.is_empty() {
                games.push_back(line);
            }
        }
        Ok(games)
    }
}

