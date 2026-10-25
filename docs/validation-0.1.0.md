# RustyCrone 0.1.0 local validation

Recorded 2026-10-25. The v0.1.0 preview release includes MIT-licensed Ubuntu and macOS downloads. The macOS assets were subsequently replaced with Developer ID–signed and Apple-notarized copies. Every file-backed Mach-O code/data section was verified unchanged from the originally tested executable; only distribution signing and license resources changed.

## Automated checks

On the macOS test machine (macOS, Apple silicon) and the Linux test machine (Ubuntu 26.04.1 LTS, x86_64): JavaScript syntax, Rust formatting, Clippy with warnings rejected, all 12 Rust tests, all five simulated-interface tests, and the Tauri debug build passed. The tests do not change real crontabs.

The optimized macOS app and Ubuntu `.deb` both built successfully. macOS signature verification passed with an ad-hoc signature; linked libraries were system libraries. Ubuntu package inspection confirmed the application-menu entry, icons, regular executable permissions, runtime dependencies, and no crontab-changing installation scripts. All shared libraries resolved on the Linux test machine. Simulated installation required only the RustyCrone package.

## User-confirmed workflows

| Build | Confirmed |
| --- | --- |
| macOS development app on the macOS test machine | User/root loading, adding, editing, and deleting harmless jobs; minute-interval, monthly, and startup previews; readable schedule descriptions |
| Standalone macOS app on the macOS test machine | Opening directly, loading the user crontab, root saving and deletion after administrator approval |
| Linux debug app on the Linux test machine | Visible window; user/root loading; user/root adding, editing, and deletion with administrator authorization |
| Installed Ubuntu package on the Linux test machine | Installation, launch from the application menu, user loading, administrator cancellation, root saving and deletion |

The user confirmed removal of the test root jobs from the standalone macOS app and installed Ubuntu app. The temporary commands used `/usr/bin/true`, which performs no work.

## Limits

- Public macOS packages are Apple silicon only, Developer ID signed, notarized, and stapled. Local `npm run build:mac` builds still use ad-hoc signing. Full Disk Access may be required separately from administrator approval.
- Ubuntu package was built and tested on 26.04. Older distributions and other Linux desktops/architectures are not validated.
- GitHub checks cover macOS and Ubuntu 24.04. Verify their current results in the repository Actions tab.
- Scheduling previews and saved entries were tested; actual timed execution and startup execution were not independently observed.
- Complex cron expressions are preserved and displayed but cannot yet be edited in the form. There is no backup-restore interface yet.
- Crontab has no atomic conditional-write operation. The app rechecks before saving, but simultaneous edits from another tool should be avoided.

The local `dist/` folder preserves the original tested artifacts. `dist/release-v0.1.0/` contains the GitHub preview-release downloads and checksums. The macOS ZIP includes LICENSE alongside the unchanged signed app; the Ubuntu package adds `/usr/share/doc/rusty-crone/copyright`, with every original installed payload verified byte-for-byte unchanged. Source archives for the MIT-licensed tag are provided by GitHub. Build commands are documented in the README.

## Notarized macOS distribution verification

On 2026-10-25, Apple accepted both the signed app and signed DMG. Stapler validation, strict/deep app signature verification, and Gatekeeper assessment passed; Gatekeeper reported `source=Notarized Developer ID`. The final DMG was mounted read-only, and the mounted app passed the same checks with the expected icon and MIT resource. The mounted app was opened in a real window: loading the user crontab returned the empty-table message, and previewing `/usr/bin/true` produced the correct hourly entry. No schedule was saved during this smoke test. The final ZIP was separately extracted and checked to confirm its stapled ticket survived packaging.
