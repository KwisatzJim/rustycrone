# RustyCrone 0.1.0 local validation

Recorded 2026-10-25. The v0.1.0 preview release packages the tested executables with the MIT license notice. The license packaging does not change application executable bytes.

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

- macOS package is Apple silicon only, ad-hoc signed, and not notarized. Full Disk Access may be required separately from administrator approval.
- Ubuntu package was built and tested on 26.04. Older distributions and other Linux desktops/architectures are not validated.
- GitHub checks cover macOS and Ubuntu 24.04. Verify their current results in the repository Actions tab.
- Scheduling previews and saved entries were tested; actual timed execution and startup execution were not independently observed.
- Complex cron expressions are preserved and displayed but cannot yet be edited in the form. There is no backup-restore interface yet.
- Crontab has no atomic conditional-write operation. The app rechecks before saving, but simultaneous edits from another tool should be avoided.

The local `dist/` folder preserves the original tested artifacts. `dist/release-v0.1.0/` contains the GitHub preview-release downloads and checksums. The macOS ZIP includes LICENSE alongside the unchanged signed app; the Ubuntu package adds `/usr/share/doc/rusty-crone/copyright`, with every original installed payload verified byte-for-byte unchanged. Source archives for the MIT-licensed tag are provided by GitHub. Build commands are documented in the README.
