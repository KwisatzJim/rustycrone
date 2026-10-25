# Linux desktop validation

Status as of 2026-10-25: code checks, all 12 Rust tests and five simulated-interface tests, and the Tauri debug build passed on the Ubuntu test machine, Ubuntu 26.04.1 LTS (x86_64). Installed tools: Rust 1.99.1, Node 24.22.1, npm 11.19.1, WebKitGTK 2.52.6, and GTK 3.24.52. The user also confirmed the visible window, user/root crontab loading, administrator authorization, and adding, editing, and deleting the harmless `/usr/bin/true` test job in both crontabs. Both Linux test jobs were removed successfully. The build required no additional system dependencies; the RustyCrone package was installed afterward as described below. Automated GitHub checks are also prepared for Ubuntu 24.04, but have not run there. Authentication-dialog cancellation was subsequently confirmed in the installed release app.

The debug executable is `src-tauri/target/debug/rustycrone` inside the Linux build folder. Run it from a terminal inside the desktop session, as the normal user, for the first manual check below.

## Ubuntu installer

A release-mode `.deb` was built on the Ubuntu test machine at `./RustyCrone_0.1.0_amd64.deb`. Package inspection confirmed a normal (not setuid) `/usr/bin/rustycrone`, the application-menu entry, and icons. Dependencies include `cron`, `pkexec`, WebKitGTK 4.1, and GTK 3. An `apt-get --simulate install` check found all dependencies already satisfied on the Ubuntu test machine and proposed only the new `rusty-crone` package. All linked libraries resolved. The user subsequently confirmed installation, opening RustyCrone from the applications menu, and loading the current user's crontab. The user subsequently confirmed cancelling administrator authentication, saving a harmless root test job, and deleting that job through administrator prompts in the installed release app.

To repeat the installation check, install this local package on the Ubuntu test machine, open **RustyCrone** from the applications menu, and load the current user's crontab:

```sh
sudo apt install ./RustyCrone_0.1.0_amd64.deb
```

The package does not contain scripts that change crontabs. The installed GUI runs as your normal user; administrator changes still use the system prompt. This package targets the Ubuntu test machine's Ubuntu 26.04 system; older Ubuntu versions are not validated.

## Build prerequisites

Use a normal desktop user account with Rust (stable), Node.js 24 or newer, and npm. Install the Tauri 2 system prerequisites for your distribution: https://v2.tauri.app/start/prerequisites/#linux.

For Ubuntu 24.04:

```sh
sudo apt-get update
sudo apt-get install -y libwebkit2gtk-4.1-dev build-essential curl wget file libxdo-dev libssl-dev librsvg2-dev libayatana-appindicator3-dev
```

The system also needs `/usr/bin/crontab`, a running cron service, `/usr/bin/pkexec`, and a working desktop PolicyKit authentication agent. Package names vary by distribution. The app does not fall back to a terminal password prompt. Do not run the GUI using sudo.

From the project directory:

```sh
npm ci
npm run check
npm test
npm run build:check
npm run tauri dev
```

## First manual check

Open the window as your normal user, select **Current user**, and click **Load crontab**. Confirm either your existing schedules or the empty-crontab message appears. This step only reads the crontab.

After the first check passes, verify a harmless `/usr/bin/true` job can be previewed, saved, reloaded, edited, and deleted. Then test root loading and administrator-dialog cancellation before making a root test entry. Each successful change must report a backup and verification. Root backups must remain root-owned and private under `/var/lib/rustycrone/backups`.

Record the distribution, version, desktop environment, session type (Wayland/X11), and any error text when reporting results.
