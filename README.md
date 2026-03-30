# 🚀 Drop_Sentry

[![Discord](https://img.shields.io/discord/1437005378750775359?style=for-the-badge&logo=discord&label=Join%20Discord)](https://discord.gg/7H7n4RPtJG)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/Rust-000000?style=for-the-badge&logo=rust&logoColor=white)](https://www.rust-lang.org)
[![Version](https://img.shields.io/badge/Version-1.0.0-success?style=for-the-badge)](https://github.com/this-is-really/TwitchDropSentryMulti/releases)

**Next-level multi-account Twitch Drops farmer.**  
Watch streams and claim time-based drops **for all your accounts at once** - completely hands-free, blazing fast, and extremely lightweight.

---

> [!IMPORTANT]  
> **DropSentry 1.0.0 - Stable Release!**  
> After extensive testing and your valuable feedback, I’m fully confident: the code is production-ready.  
> No more beta status. From this version onward, the project is officially **stable**.  

## ✨ Why DropSentry Stands Out

- **True multi-account support** - run as many Twitch accounts as you want simultaneously  
- **Smart game priority system** - just list your games; the higher in the file, the higher the priority  
- **Proxy support** - dedicated proxy list for maximum privacy and safety  
- **Autostart + fully customizable config**  
- **Beautiful real-time UI** with per-account progress bars  
- **Auto-claim + anti-duplicate protection**  
- **Lightweight & fast** - pure Rust, no browser, no bloat  

![DropSentry Interface](assets/bars.gif)

**This is the evolved multi-account fork** of the original TwitchDropSentry, built for real drop farmers.

## 🚀 Quick Start (30 seconds)

1. Download the latest release from [Releases](https://github.com/this-is-really/TwitchDropSentryMulti/releases)
2. Run `twitchdrops_miner.exe` (Windows) or `./twitchdrops_miner` (Linux)
3. Log in to all your accounts (sessions are saved automatically)
4. Done - the tool will create the `lists/` folder and start farming right away

## ⚙️ Configuration (since 1.0.0)

All settings are now in one clean file:

**`data/config.json`**

```json
{
  "games_path": "./lists/games.txt",
  "autostart": false,
  "proxies_path": "./lists/proxies.txt"
}
```

### What the program does automatically

- On first launch it creates the `lists/` folder and the necessary files inside
- You can point it to your own custom paths if you prefer

### `lists/games.txt` (priority from top to bottom)

```txt
THE FINALS
Marvel Rivals
Warhammer 40,000: Darktide
Rust
Valorant
```

**The higher the game is in the list — the higher its priority.**  
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
2. Fetches current Drop campaigns  
3. For each account picks the highest-priority eligible game  
4. Finds the best live stream for that game  
5. Emulates real viewing via official Twitch GQL  
6. Shows beautiful real-time progress for every account  
7. Automatically claims drops and saves history to prevent duplicates

## 📥 Installation

**Pre-built binaries:**

- **Windows** → `twitchdrops_miner.exe` (x86_64)
- **Linux** → `twitchdrops_miner` (x86_64 ELF)

## 💾 Data & Security

All sessions and data are stored as plain JSON files in the `data/` folder.  
**Recommendation:** Use farming-only accounts and always enable proxies.

We are not responsible for bans or data leaks — use at your own risk.

## 🐞 Bug Reports

Found a bug (critical or minor)? Open an **Issue** right away.  
Every report helps make the project even better.

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
</div>

---

**Made with ❤️ for the Twitch community**  
**License:** [MIT](LICENSE)  
**Version:** 1.0.0 Stable
