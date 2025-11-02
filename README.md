# 🚀 Drop_Sentry (0.1.0 Multi-Account Beta) Release Notes

> [!NOTE]
>
> ## 🚀 Multi-Account Beta Release Notice **0.1.0**
>
> This is a **Beta release** **0.1.0** focused on introducing **multi-account support**. The core drop-claiming functionality is forked from the stable **0.3.0-rs.1** codebase, but the multi-account management is **new and currently unstable**.
>
> Your main purpose in testing this version is to:
>
> * **Stress-test** the multi-account login, session management, and concurrent claiming logic.
> * Find any new **bugs** or unexpected edge-cases related to managing multiple users.
> * Evaluate the **usability** of the new multi-account configuration process.
>
> **Do not use this version for critical production data, especially on your primary accounts.**
>
> **Thank you for helping us stabilize this major feature!**

-----

## What is this?

This is a **forked command-line tool** designed to automatically watch Twitch streams and claim Time-Based Drops for a selected game, with a **primary focus on managing multiple Twitch accounts concurrently**.

> 💡 **Forked from:** This project is a fork of the original [**Drop_Sentry**](https://github.com/this-is-really/TwitchDropSentry) project, focusing on the new multi-account feature set.

It runs in the background, finds eligible streams, simulates watch time by sending the necessary **GQL** events, and automatically claims drops for **all configured users** as they become available.

### How it Works (New Multi-Account Focus)

1. **Supports multiple accounts:** You can now log into and manage several Twitch accounts simultaneously.
2. Logs in to **all configured Twitch accounts** (saves credentials to `data/{ACCOUNT_NAME}.json`).
3. Fetches active Drop Campaigns and **groups them by game** to ask you to select one.
4. For **each active account**, it finds and prioritizes the **best eligible live stream** for that campaign.
5. Simulates "watching" that stream for every account. **Note:** The underlying **GQL** implementation is powered by [**twitch-gql-rs**](https://github.com/this-is-really/twitch-gql-rs).
6. Monitors drop progress with a **real-time terminal progress bar** for each user/drop.
7. **Automatically claims** the drop once the required time is met, with robust retry logic.
8. Saves claimed drops to `data/cash.json` (per-account logging) to avoid re-claiming.

-----

## 💻 Available Binaries

Standard pre-compiled binaries are provided for common platforms.

* **Windows:** Executable for **x86_64** architecture **.exe** file.
* **Linux:** **ELF** executable for **x86_64** architecture.

-----

## 🐞 Found a Bug?

This **Beta release is highly focused on multi-account stability**. If you encounter *any* crashes, errors, or unexpected behavior-**especially** when managing multiple users—please **open an Issue** in this repository immediately. Your feedback is crucial for stabilizing this core feature.

-----

## 🎉 Did you like the app?

Please consider rating this repository by clicking the star in the top-right corner of the page on GitHub (you need to be logged into your account). This gives me the motivation to keep developing this project.

![Star](https://i.ibb.co/3YkyqJQ8/2025-10-31-20-25.png)

-----

## ❤️ Support the Developer

<div align="center">

[![DonationAlerts](https://www.donationalerts.com/img/brand/donationalerts.svg)](https://www.donationalerts.com/r/this_is_really)

Your support will accelerate development and help ensure the long-term maintenance of this project
