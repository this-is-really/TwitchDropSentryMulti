# 🚀 Drop_Sentry (Multi-Account Beta)

> [!WARNING]
> **This is a Beta release (0.1.0+)** focused on **multi-account support**.  
> The core drop-claiming logic is stable (forked from 0.3.0-rs.1), but multi-account login, session management, and concurrent claiming are **new and still experimental**.  
> 
> **Use this version only for testing.** Do not run it on your primary accounts or with valuable data.

**Your help is extremely valuable:** stress-test multi-account functionality, report bugs, and help us stabilize this major feature.

---

[![Discord](https://img.shields.io/discord/1437005378750775359?style=for-the-badge&logo=discord&label=Discord)](https://discord.gg/7H7n4RPtJG)

## What is Drop_Sentry?

**Drop_Sentry** is a powerful command-line tool that automatically watches Twitch streams and claims **Time-Based Drops** for selected games — now with full **multi-account support**.

It runs in the background, finds eligible live streams, simulates watching time by sending the required GQL events, and claims drops for **all configured accounts** as soon as they become available.

> 💡 **This is a multi-account fork** of the original [Drop_Sentry](https://github.com/this-is-really/TwitchDropSentry) project.

### ✨ Key Features
- Simultaneous support for **multiple Twitch accounts**
- Automatic login and persistent session management
- Smart grouping of Drop Campaigns by game
- Intelligent selection of the best eligible live stream per account
- Real-time terminal progress bars for every user and drop
- Automatic claiming with robust retry logic
- Per-account drop history (`data/cash.json`)
- Configurable game selection and autostart
- Powered by [**twitch-gql-rs**](https://github.com/this-is-really/twitch-gql-rs)

## How It Works

1. Logs into **all configured Twitch accounts** (sessions saved in `data/{ACCOUNT_NAME}.json`)
2. Fetches active Drop Campaigns and groups them by game
3. Lets you select a game (or uses the one specified in config)
4. For each account, finds and joins the best eligible live stream
5. Simulates watching by sending GQL events
6. Shows real-time progress for every account
7. Automatically claims the drop when requirements are met
8. Saves claim history to prevent re-claiming

## Configuration (since 0.1.3-beta)

You can now customize behavior with a simple JSON config.

Example (`data/config.json`):

```json
{
  "game": "Rust",
  "autostart": true
}
```

- `"game"`: If set to a non-empty string, the tool **skips the interactive menu** and immediately starts farming drops for that game.
- `"autostart"`: If `true`, the miner registers itself as a system startup application and launches automatically with Windows/Linux.

## 💻 Available Binaries

Pre-compiled executables are provided for the most common platforms:

- **Windows** — `x86_64` `.exe`
- **Linux** — `x86_64` ELF executable

## Data Storage & Important Disclaimer

All account credentials, sessions, and claim data are stored in **plain JSON files** inside the `data/` folder.

**We are not responsible** for:
- Any data leaks
- Twitch account bans due to suspicious activity
- Any other consequences of using this tool

**Use at your own risk.** Never run this on your main/primary accounts with valuable data.

## 🐞 Found a Bug?

This beta version is heavily focused on multi-account stability.  
If you encounter **any** crashes, errors, or unexpected behavior (especially with multiple accounts), please **open an Issue** immediately. Your feedback is crucial.

## 🎉 Did you like the project?

If Drop_Sentry is useful to you, please consider **starring the repository** ⭐  
It really helps the project grow and motivates further development.

## ❤️ Support the Developer

<div align="center">

[![DonationAlerts](https://www.donationalerts.com/img/brand/donationalerts.svg)](https://www.donationalerts.com/r/this_is_really)

**[Boosty](https://boosty.to/this-is-really)**

Your support greatly accelerates development and helps ensure long-term maintenance of the project.

</div>
