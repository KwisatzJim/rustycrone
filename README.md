# RustyCrone

A desktop GUI for managing user and root crontabs on macOS and Linux, built with Rust and Tauri 2.

Download the macOS Apple silicon ZIP or Ubuntu 26.04 amd64 installer from [GitHub Releases](https://github.com/KwisatzJim/rustycrone/releases). The macOS build is ad-hoc signed and is not notarized.

![RustyCrone icon](src-tauri/icons/128x128.png)

## Features

- Select the current user's or root's crontab.
- Read schedules as individual rows with plain-language descriptions and the original cron expression.
- Add, edit, preview, and delete jobs.
- Choose minute intervals, hourly, daily, weekly, monthly, or startup schedules.
- Create a private backup before each change, detect changes since loading, and verify the installed crontab.
- Use the system administrator dialog for root access. Passwords are never collected or stored by the app.
- Preserve comments, environment settings, and other entries when changing a selected job.

Complex expressions remain visible and can be deleted after confirmation, but the form only edits its supported schedule types. This app manages per-user crontabs; it does not edit `/etc/crontab`, `/etc/cron.d`, launchd jobs, or systemd timers.

## Development

Install Rust, Node.js 24 or newer, and the [Tauri 2 platform prerequisites](https://v2.tauri.app/start/prerequisites/). Then:

```sh
npm ci
npm run tauri dev
```

Rust commands live in `src-tauri/src/`. The interface uses HTML, CSS, and JavaScript in `src/`, without a frontend framework or development web server.

## Using the app

Choose **Current user** or **Root**, then click **Load crontab**. Loading only reads schedules. Root access opens the system administrator dialog when required by system policy.

To add a job, enter a command, select a schedule, and click **Preview job**. Review the entry before saving. Saving installs the schedule; cron can then execute the command at its scheduled time. Use full paths because cron has a limited environment. Percent signs have special meaning in cron commands.

**Edit…** fills the form with an existing supported job. Saving replaces only that entry in its current position. **Delete…** shows the exact entry and asks for confirmation. Deleting a schedule does not stop an already-running command.

Minute intervals support 1, 5, 10, 15, 20, or 30 minutes, aligned to minute 0 of each hour. Monthly dates 29–31 skip months without that date. **At startup (cron)** creates an `@reboot` entry, which runs when cron starts, usually during boot; it is not a user-login task. Network connections and other services may not yet be ready.

### Administrator access

The GUI runs as your normal user. Root reads use macOS AppleScript authorization or Linux PolicyKit (`pkexec`). Root changes run a short-lived helper that checks, backs up, installs, and verifies root's crontab. Job data is passed in a private temporary JSON file, not inserted into a shell command or executed during saving. The temporary request is removed when the operation finishes; a crash may leave a private temporary folder.

On Linux, `/usr/bin/crontab`, `/usr/bin/pkexec`, a running cron service, and a desktop PolicyKit authentication agent are required. There is no terminal-password fallback. Do not run the GUI with sudo.

On macOS, administrator approval and Full Disk Access are separate permissions. If saving reports `crontab: tmp/tmp.…: Operation not permitted`, add RustyCrone under **System Settings → Privacy & Security → Full Disk Access**, then quit and reopen it. For a development build, select `src-tauri/target/debug/rustycrone`; the error displays its full path. The standalone app may need a separate grant. Do not change cron-storage permissions to work around this error.

### Backups

Each successful change reports the backup path. User backups are stored in the platform app-data directory under `com.jimkelley.rustycrone/backups`. Root backups are private to root:

- macOS: `/var/root/Library/Application Support/com.jimkelley.rustycrone/backups`
- Linux: `/var/lib/rustycrone/backups`

Empty backups represent previously empty crontabs. There is no backup-restore interface yet. Crontab provides no atomic conditional write; avoid simultaneous edits from other tools even though the app checks for changes immediately before installation.

## Build local packages

On macOS:

```sh
npm run build:mac
```

The app is created at `src-tauri/target/release/bundle/macos/RustyCrone.app`. This configuration uses ad-hoc signing for local testing. It is not Developer ID signed or notarized for distribution. The tested build is Apple silicon only.

On Ubuntu:

```sh
npm run build:deb
```

The installer is created under `src-tauri/target/release/bundle/deb/`. Install the generated file using `sudo apt install ./RustyCrone_0.1.0_amd64.deb`. It includes the application-menu entry and icons and declares cron, pkexec, and desktop-library dependencies. Building does not install anything or change schedules. The tested package was built on Ubuntu 26.04.1 LTS; older distributions are not validated.

Node and npm are not required to run either packaged app. Installers and SHA-256 checksums are distributed through GitHub Releases; generated files remain outside the source history.

## Checks and validation

```sh
npm run check
npm test
npm run build:check
```

These check JavaScript syntax, Rust formatting and Clippy warnings, run 12 Rust tests and five simulated-interface tests, and build the desktop app without an installer. Tests never modify real crontabs or request administrator access.

The macOS and Ubuntu GitHub workflow runs these checks on pushes and pull requests. It does not publish releases or install cron jobs.

User and root workflows were verified in real windows on macOS and Ubuntu 26.04.1 LTS. Standalone-package checks included root saving and deletion on both systems, plus authentication cancellation on Ubuntu. See [validation notes](docs/validation-0.1.0.md) and [Linux testing](docs/linux-testing.md) for the exact scope. Actual timed/startup execution has not been independently observed.

## License

MIT. See [LICENSE](LICENSE).
