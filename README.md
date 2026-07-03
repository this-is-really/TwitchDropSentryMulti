# 🚀 DropSentry
[![Discord](https://img.shields.io/discord/1437005378750775359?style=for-the-badge&logo=discord&label=Join%20Discord)](https://discord.gg/7H7n4RPtJG)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/Rust-000000?style=for-the-badge&logo=rust&logoColor=white)](https://www.rust-lang.org)
[![Version](https://img.shields.io/badge/Version-1.0.5-success?style=for-the-badge)](https://github.com/this-is-really/TwitchDropSentryMulti/releases)

**Next-level multi-account Twitch Drops farmer.**
Watch streams and claim time-based drops **for all your accounts at once** - completely hands-free, blazing fast, and extremely lightweight.

---
> [!IMPORTANT]
> **DropSentry 1.0.5 - Stability and diagnostics**
>
> Improved error handling and logging now cover startup, stream watching, campaign handling, and drop retrieval. Failed client loads are handled more safely, with invalid state moved to `delete_accounts/` instead of crashing.
>
> Use the dedicated `--debug` mode documented below for detailed diagnostics and bug reports.

## ✨ Why DropSentry Stands Out
- **True multi-account support** - run as many Twitch accounts as you want simultaneously
- **Smart game priority system** - just list your games; the higher in the file, the higher the priority
- **Proxy support** - dedicated proxy list for maximum privacy and safety
- **Discord Webhook notifications** - real-time alerts for drop claims, farming status and more
- **Automatic account validation** - dead accounts are detected and cleaned up automatically
- **Autostart + fully customizable config**
- **Beautiful real-time UI** with per-account progress bars
- **Auto-claim + anti-duplicate protection**
- **Lightweight & fast** - pure Rust, no browser, no bloat

![DropSentry Interface](assets/bars.gif)

**This is the evolved multi-account fork** of the original [TwitchDropSentry](https://github.com/this-is-really/TwitchDropSentry), built for real drop farmers.

## 🚀 Quick Start (30 seconds)
1. Download the latest release from [Releases](https://github.com/this-is-really/TwitchDropSentryMulti/releases)
2. **Windows**: simply run `twitchdrops_miner.exe`  
   **Linux**: first make the binary executable with  
   ```bash
   chmod +x twitchdrops_miner-linux-x86_64
   ```
   then run `./twitchdrops_miner-linux-x86_64`
3. Log in to all your accounts (sessions are saved automatically)
4. Done - the tool will create the `lists/` folder (if it doesn't exist) and start farming right away

## ⚙️ Configuration (since 1.0.1)
All settings are now in one clean file:  
**`data/config.json`**

```json
{
  "games_path": "./lists/games.txt",
  "proxies_path": "./lists/proxies.txt",
  "autostart": false,
  "discord_webhook_url": ""
}
```

### Discord Webhook Support (since 1.0.1)
- New field: **`discord_webhook_url`**
- Paste your Discord Webhook URL to enable notifications.
- If left empty (`""`), notifications will be completely disabled.

**Example with webhook enabled:**
```json
{
  "games_path": "./lists/games.txt",
  "proxies_path": "./lists/proxies.txt",
  "autostart": false,
  "discord_webhook_url": "https://discord.com/api/webhooks/1488241374007660735/VndLpLuv6iE_jBiNwVBmsIoiSA8pDNsxssjsCxDs_DaA-U2fwrge7wPfSFctVwSAo1X_"
}
```

**Notifications preview:**  
![Webhook notifications](assets/webhook.png)

### Account Validation (NEW in 1.0.4)
- DropSentry checks the health of every account on startup and periodically while farming.
- A real request is sent to Twitch for each account - if Twitch responds with an error (ban, expired session, invalid credentials), the account is flagged as invalid.
- Invalid accounts are automatically moved to the **`delete_accounts/`** folder, so your active accounts list stays clean without any manual intervention.
- Farming continues uninterrupted for all remaining valid accounts.

### What the program does automatically
- On first launch it creates the `lists/` folder and the necessary files inside
- You can point it to your own custom paths if you prefer
- The `discord_webhook_url` field is automatically added to existing configs
- Dead/invalid accounts are detected and moved to `delete_accounts/` automatically

### `lists/games.txt` (priority from top to bottom)
```txt
THE FINALS
Marvel Rivals
Warhammer 40,000: Darktide
Rust
Valorant
```
**The higher the game is in the list - the higher its priority.**  
The tool will first try to find a stream for the top game, then the next, and so on.

### `lists/proxies.txt` (one proxy per line)
```txt
socks5://user:pass@123.45.67.89:1080
http://192.168.0.1:8080
socks5://2esfs:323e@192.168.0.1:8000
```
Fully supports HTTP and SOCKS5 (with or without authentication).

## How It Works
1. Logs into **all** configured Twitch accounts
2. Validates each account's health via the Twitch API and quarantines dead ones
3. Fetches current Drop campaigns
4. For each account picks the highest-priority eligible game
5. Finds the best live stream for that game (or waits patiently if none is live yet)
6. Emulates real viewing via official Twitch GQL
7. Shows beautiful real-time progress for every account
8. Automatically claims drops and saves history to prevent duplicates
9. Sends Discord webhook notifications when configured

## 📥 Pre-built Binaries & Builds

DropSentry provides **official pre-compiled binaries** for all major platforms.  
All builds are available on the [Releases page](https://github.com/this-is-really/TwitchDropSentryMulti/releases).

### Supported Platforms

| Operating System       | Architecture              | Binary Name                                      | Notes |
|------------------------|---------------------------|--------------------------------------------------|-------|
| **Windows**            | x86_64                    | `twitchdrops_miner-windows-x86_64.exe`          | Double-click to run |
| **Linux**              | x86_64                    | `twitchdrops_miner-linux-x86_64`                | Requires `chmod +x` |
| **Linux**              | aarch64 (ARM64)           | `twitchdrops_miner-linux-aarch64`               | Requires `chmod +x` |
| **macOS**              | Apple Silicon (aarch64)   | `twitchdrops_miner-macos-aarch64`               | Requires `chmod +x` |
| **macOS**              | Intel (x86_64)            | `twitchdrops_miner-macos-x86_64`                | Requires `chmod +x` |

**Quick command for Linux / macOS:**
```bash
chmod +x twitchdrops_miner-*
```

## 💾 Data & Security
All sessions and data are stored as plain JSON files in the `data/` folder.  
**Recommendation:** Use farming-only accounts and always enable proxies.  
We are not responsible for bans or data leaks - use at your own risk.

## 🐞 Bug Reports
Found a bug (critical or minor)? Open an **Issue** right away.  
Every report helps make the project even better.

## 🛠️ Debug Mode (`--debug`)
- Run the binary with `--debug` to enable verbose diagnostic logging for startup, account validation, stream watching, and drop retrieval.
- This mode is the best way to capture full runtime details when troubleshooting.
- When reporting an issue, include:
  - the full `--debug` console output
  - the exact command you used
  - any relevant `data/config.json` settings
  - files moved to `delete_accounts/`

## ⭐ Support the Project
If DropSentry is helping you farm drops, please drop a **star** ⭐  
It’s the best motivation to keep pushing updates.

## ❤️ Support the Developer
<div align="center">
  <a href="https://www.donationalerts.com/r/this_is_really">
    <img src="https://www.donationalerts.com/img/brand/donationalerts.svg" height="45">
  </a>
  <br><br>
  <a href="https://boosty.to/this-is-really">Boosty</a>
  <br><br>
  <b>USDT (TRC20):</b> <code>TRiVzMYpcDovpw2g614FGGzFR9Ws6gXqgU</code>
</div>

---
**Made with ❤️ for the Twitch community**  
**License:** [MIT](LICENSE)  
**Version:** 1.0.5
