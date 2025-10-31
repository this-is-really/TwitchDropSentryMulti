# 🚀 Drop_Sentry (0.2.0 Beta) Release Notes

> [!NOTE]
>
> ### 🚀 Beta Release Notice ($\mathbf{0.2.0}$)
>
> This is a **Beta release** ($\mathbf{0.2.0}$). The major feature set is **complete**, and I have fixed most critical bugs found during the Alpha phase.
>
> Your main purpose in testing this version is to:
>
> * Find any remaining **minor bugs** and unexpected edge-cases.
> * Evaluate **usability** and overall user experience.
> * Test the application under **real-world conditions**.
>
> While this version is significantly more stable than Alpha, it may still contain bugs that could affect data or performance. **Do not use this version for critical production data.**
>
> **Thank you for your feedback!**

---

## What is this?

This is a command-line tool designed to automatically watch Twitch streams and claim Time-Based Drops for a selected game.

It runs in the background, finds eligible streams, simulates watch time by sending the necessary $\text{GQL}$ events, and automatically claims drops as they become available.

### How it Works

1. Logs into your Twitch account (saves credentials to `data/save.json`).
2. Fetches active Drop Campaigns and **groups them by game** to ask you to select one.
3. Finds and prioritizes the **best eligible live stream** for that campaign.
4. Simulates "watching" that stream. **Note:** The underlying $\text{GQL}$ implementation is powered by [**twitch-gql-rs**](https://github.com/this-is-really/twitch-gql-rs).
5. Monitors your drop progress with a **real-time terminal progress bar**.
6. **Automatically claims** the drop once the required time is met, with robust retry logic.
7. Saves claimed drops to `data/cash.json` to avoid re-claiming.

## 💻 Available Binaries

Standard pre-compiled binaries are provided for common platforms.

* **Windows:** Executable for **x86\_64** architecture (**$\text{.exe}$** file).
* **Linux:** **$\text{ELF}$** executable for **x86\_64** architecture.

## 🐞 Found a Bug?

Bugs were common during the Alpha stage, but this **Beta release is significantly more stable**. We've fixed most critical issues, and you might have to try hard now to find the remaining errors.

If you still encounter *any* crashes, errors, or unexpected behavior, please **open an Issue** in this repository.

## :tada: Did you like the app?

Please consider rating this repository by clicking the star in the top-right corner of the page on GitHub (you need to be logged into your account). This gives me the motivation to keep developing this project.

![Star](https://i.ibb.co/3YkyqJQ8/2025-10-31-20-25.png)

## ❤️ Support the Developer

<div align="center">

[![DonationAlerts](https://www.donationalerts.com/img/brand/donationalerts.svg)](https://www.donationalerts.com/r/this_is_really)

Your support will accelerate development and help ensure the long-term maintenance of this project.
