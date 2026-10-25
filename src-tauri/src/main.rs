#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod root;

use std::io::Write;
use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt};
use std::process::{Command, Stdio};
use std::sync::Mutex;
use tauri::Manager;

static SAVE_LOCK: Mutex<()> = Mutex::new(());

fn is_job(line: &str) -> bool {
    let fields: Vec<_> = line.split_whitespace().collect();
    if fields.first().is_some_and(|field| {
        [
            "@reboot",
            "@yearly",
            "@annually",
            "@monthly",
            "@weekly",
            "@daily",
            "@midnight",
            "@hourly",
        ]
        .contains(field)
    }) {
        return fields.len() >= 2;
    }
    fields.len() >= 6
        && fields[..5].iter().all(|field| {
            !field.is_empty()
                && field
                    .chars()
                    .all(|c| c.is_ascii_digit() || "*/,-".contains(c))
        })
}

fn remove_job(current: &str, line_index: usize) -> Result<String, String> {
    let lines: Vec<_> = current.split_inclusive('\n').collect();
    let line = lines
        .get(line_index)
        .ok_or("The selected job no longer exists. Reload your crontab.")?;
    if !is_job(line) {
        return Err(
            "Only recognized job lines can be deleted. Comments and settings are preserved.".into(),
        );
    }
    Ok(lines
        .iter()
        .enumerate()
        .filter_map(|(index, line)| (index != line_index).then_some(*line))
        .collect())
}

fn replace_job(current: &str, line_index: usize, entry: &str) -> Result<String, String> {
    let without_job = remove_job(current, line_index)?;
    // Reuse schedule validation and check duplicates against every other job.
    append_job(&without_job, entry)?;
    Ok(current
        .split_inclusive('\n')
        .enumerate()
        .map(|(index, line)| {
            if index == line_index {
                let ending = if line.ends_with("\r\n") { "\r\n" } else { "\n" };
                format!("{entry}{ending}")
            } else {
                line.to_string()
            }
        })
        .collect())
}

fn append_job(current: &str, entry: &str) -> Result<String, String> {
    if entry.contains(['\r', '\n', '\0']) {
        return Err("A job must be a single line.".into());
    }
    if let Some(command) = entry.strip_prefix("@reboot ") {
        if command.trim().is_empty() {
            return Err("Enter a command for the startup job.".into());
        }
    } else {
        let fields: Vec<_> = entry.splitn(6, ' ').collect();
        if fields.len() != 6 || fields[5].trim().is_empty() {
            return Err("Preview a complete schedule and command before saving.".into());
        }
        let interval = fields[0]
            .strip_prefix("*/")
            .and_then(|value| value.parse::<u8>().ok());
        let minute_interval = interval.is_some_and(|n| [1, 5, 10, 15, 20, 30].contains(&n));
        for (index, (field, maximum)) in fields[..5].iter().zip([59, 23, 31, 12, 6]).enumerate() {
            if index == 0 && minute_interval {
                continue;
            }
            if *field != "*" && !field.parse::<u8>().is_ok_and(|n| n <= maximum) {
                return Err("The schedule contains an invalid field.".into());
            }
        }
        let monthly = fields[2] != "*";
        // Only the schedule shapes exposed by the form; don't combine month days and weekdays.
        if fields[3] != "*"
            || fields[0] == "*"
            || (fields[1] == "*" && fields[4] != "*")
            || (minute_interval && (fields[1] != "*" || fields[4] != "*" || monthly))
            || (monthly
                && (!fields[2].parse::<u8>().is_ok_and(|n| n >= 1)
                    || fields[1] == "*"
                    || fields[4] != "*"))
        {
            return Err(
                "Choose a minute interval, hourly, daily, weekly, or monthly schedule.".into(),
            );
        }
    }
    if current.lines().any(|line| line.trim() == entry.trim()) {
        return Err("This exact job already exists in your crontab.".into());
    }
    let separator = if current.is_empty() || current.ends_with('\n') {
        ""
    } else {
        "\n"
    };
    Ok(format!("{current}{separator}{entry}\n"))
}

#[tauri::command]
async fn save_user_job(
    app: tauri::AppHandle,
    expected: String,
    entry: String,
) -> Result<String, String> {
    change_user_table(app, expected, move |current| append_job(current, &entry)).await
}

#[tauri::command]
async fn delete_user_job(
    app: tauri::AppHandle,
    expected: String,
    line_index: usize,
) -> Result<String, String> {
    change_user_table(app, expected, move |current| {
        remove_job(current, line_index)
    })
    .await
}

#[tauri::command]
async fn edit_user_job(
    app: tauri::AppHandle,
    expected: String,
    line_index: usize,
    entry: String,
) -> Result<String, String> {
    change_user_table(app, expected, move |current| {
        replace_job(current, line_index, &entry)
    })
    .await
}

async fn change_user_table(
    app: tauri::AppHandle,
    expected: String,
    change: impl FnOnce(&str) -> Result<String, String> + Send + 'static,
) -> Result<String, String> {
    let directory = app
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?
        .join("backups");
    tauri::async_runtime::spawn_blocking(move || {
        change_current_table(&directory, "user", &expected, change)
    })
    .await
    .map_err(|e| e.to_string())?
}

// The privileged helper calls this as root; "user" then means root's own crontab.
fn change_current_table(
    directory: &std::path::Path,
    owner: &str,
    expected: &str,
    change: impl FnOnce(&str) -> Result<String, String>,
) -> Result<String, String> {
    let _guard = SAVE_LOCK
        .try_lock()
        .map_err(|_| "Another save is in progress.".to_string())?;
    let current = read_table("user")?;
    if current != expected {
        return Err(
            "Your crontab changed since it was loaded. Load it again before saving.".into(),
        );
    }
    let updated = change(&current)?;
    std::fs::DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(directory)
        .map_err(|e| format!("Could not create backup folder: {e}"))?;
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_nanos();
    let backup = directory.join(format!("{owner}-{timestamp}.crontab"));
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(&backup)
        .map_err(|e| format!("Could not create backup: {e}"))?;
    file.write_all(current.as_bytes())
        .and_then(|_| file.sync_all())
        .map_err(|e| format!("Could not finish backup; nothing was saved: {e}"))?;
    // Recheck immediately before installation; crontab offers no atomic compare-and-swap.
    if read_table("user")? != current {
        return Err("Your crontab changed during backup. Load it again before saving.".into());
    }
    let mut child = Command::new("/usr/bin/crontab")
        .arg("-")
        .env("LC_ALL", "C")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("Could not start crontab: {e}"))?;
    let write_result = child
        .stdin
        .take()
        .ok_or("Could not open crontab input.")?
        .write_all(updated.as_bytes());
    let output = child
        .wait_with_output()
        .map_err(|e| format!("Could not confirm save: {e}. Reload before retrying."))?;
    if !output.status.success() {
        return Err(format!(
            "Crontab rejected the change: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    write_result
        .map_err(|e| format!("Could not send complete crontab: {e}. Reload before retrying."))?;
    if read_table("user")? != updated {
        return Err("The saved crontab could not be verified. Reload before retrying.".into());
    }
    Ok(backup.to_string_lossy().into_owned())
}

#[tauri::command]
async fn read_crontab(target: String) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || read_table(&target))
        .await
        .map_err(|error| error.to_string())?
}

fn read_table(target: &str) -> Result<String, String> {
    if target == "root" {
        return read_root_table();
    }
    let mut command = match target {
        "user" => Command::new("/usr/bin/crontab"),
        _ => return Err("Choose either the current user or root.".into()),
    };
    let output = command
        .arg("-l")
        .env("LC_ALL", "C")
        .output()
        .map_err(|error| format!("Could not run crontab: {error}"))?;
    if output.status.success() {
        return String::from_utf8(output.stdout)
            .map_err(|_| "The crontab contains text that is not valid UTF-8.".into());
    }
    let diagnostic = String::from_utf8_lossy(&output.stderr);
    // macOS prefixes this message with "crontab:", while Linux may omit it.
    let message = diagnostic.trim();
    let message = message.strip_prefix("crontab: ").unwrap_or(message);
    if message.starts_with("no crontab for ") {
        return Ok(String::new());
    }
    Err(format!(
        "Could not read your crontab: {}",
        diagnostic.trim()
    ))
}

#[cfg(target_os = "macos")]
const ROOT_READ_SCRIPT: &str = r#"try
    return do shell script "LC_ALL=C /usr/bin/crontab -u root -l" with administrator privileges without altering line endings
on error messageText number errorNumber
    if errorNumber = 1 and (messageText starts with "crontab: no crontab for root" or messageText starts with "no crontab for root") then
        return ""
    end if
    error messageText number errorNumber
end try"#;

fn root_read_result(output: std::process::Output, macos: bool) -> Result<String, String> {
    if output.status.success() {
        let text = String::from_utf8(output.stdout)
            .map_err(|_| "Root's crontab contains text that is not valid UTF-8.".to_string())?;
        // osascript appends one newline when printing the returned string.
        return Ok(if macos {
            text.strip_suffix('\n').unwrap_or(&text).to_string()
        } else {
            text
        });
    }
    let diagnostic = String::from_utf8_lossy(&output.stderr);
    if (macos && diagnostic.contains("(-128)")) || (!macos && output.status.code() == Some(126)) {
        return Err("Administrator authorization was cancelled. No changes were made.".into());
    }
    if !macos {
        let message = diagnostic.trim();
        let message = message.strip_prefix("crontab: ").unwrap_or(message);
        if output.status.code() == Some(1) && message.starts_with("no crontab for root") {
            return Ok(String::new());
        }
    }
    Err(format!("Could not read root's crontab. Administrator authorization and a working system authentication service are required.\n\n{}", diagnostic.trim()))
}

#[cfg(target_os = "macos")]
fn read_root_table() -> Result<String, String> {
    // The privileged command is fixed; no schedule or command text enters this script.
    // Avoid inheriting a project directory that macOS denies to the privileged shell.
    let output = Command::new("/usr/bin/osascript")
        .current_dir("/")
        .args(["-e", ROOT_READ_SCRIPT])
        .stdin(Stdio::null())
        .output()
        .map_err(|e| format!("Could not open the system administrator prompt: {e}"))?;
    root_read_result(output, true)
}

#[cfg(target_os = "linux")]
fn read_root_table() -> Result<String, String> {
    let output = Command::new("/usr/bin/pkexec")
        .current_dir("/")
        .args(["--disable-internal-agent", "/usr/bin/env", "LC_ALL=C", "/usr/bin/crontab", "-u", "root", "-l"])
        .stdin(Stdio::null()).output()
        .map_err(|e| format!("Could not open administrator authorization: {e}. Linux requires pkexec and a desktop PolicyKit authentication agent."))?;
    root_read_result(output, false)
}

#[cfg(not(any(target_os = "macos", target_os = "linux")))]
fn read_root_table() -> Result<String, String> {
    Err("Root crontabs are supported on macOS and Linux.".into())
}

fn main() {
    if root::run_helper_if_requested() {
        return;
    }
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            read_crontab,
            save_user_job,
            root::save_root_job,
            root::edit_root_job,
            root::delete_root_job,
            edit_user_job,
            delete_user_job
        ])
        .run(tauri::generate_context!())
        .expect("Could not start RustyCrone");
}

#[cfg(test)]
mod tests {
    use super::{append_job, remove_job, replace_job};

    #[test]
    fn supports_startup_jobs_with_existing_backup_and_edit_rules() {
        let entry = "@reboot /usr/bin/true";
        assert_eq!(
            append_job("# Keep\n", entry).unwrap(),
            "# Keep\n@reboot /usr/bin/true\n"
        );
        assert_eq!(
            replace_job("# Keep\n0 * * * * /old\n", 1, entry).unwrap(),
            "# Keep\n@reboot /usr/bin/true\n"
        );
        assert_eq!(
            replace_job("@reboot /old\n", 0, "0 * * * * /new").unwrap(),
            "0 * * * * /new\n"
        );
        assert!(append_job("@reboot /usr/bin/true\n", entry).is_err());
        for entry in [
            "@reboot ",
            "@reboot",
            "@reboot /ok\n@reboot /bad",
            "@reboot /ok\0bad",
            "@unknown /bad",
        ] {
            assert!(append_job("", entry).is_err(), "accepted {entry}");
        }
    }

    #[test]
    fn supports_monthly_dates_without_combining_weekdays() {
        for date in 1..=31 {
            let entry = format!("30 9 {date} * * /usr/bin/true");
            assert_eq!(append_job("", &entry).unwrap(), format!("{entry}\n"));
            assert_eq!(
                replace_job("# Keep\n0 * * * * /old\n", 1, &entry).unwrap(),
                format!("# Keep\n{entry}\n")
            );
        }
        for entry in [
            "30 9 0 * * /bad",
            "30 9 00 * * /bad",
            "30 9 32 * * /bad",
            "30 * 1 * * /bad",
            "30 9 1 * 1 /bad",
            "*/15 * 1 * * /bad",
        ] {
            assert!(append_job("", entry).is_err(), "accepted {entry}");
        }
    }

    #[test]
    fn supports_preset_minute_intervals_for_add_and_edit() {
        for interval in [1, 5, 10, 15, 20, 30] {
            let entry = format!("*/{interval} * * * * /usr/bin/true");
            assert_eq!(
                append_job("# Keep\n", &entry).unwrap(),
                format!("# Keep\n{entry}\n")
            );
            assert_eq!(
                replace_job("# Keep\n0 * * * * /old\n", 1, &entry).unwrap(),
                format!("# Keep\n{entry}\n")
            );
        }
        for entry in [
            "*/0 * * * * /bad",
            "*/7 * * * * /bad",
            "*/60 * * * * /bad",
            "*/15 9 * * * /bad",
            "*/15 * * * 1 /bad",
        ] {
            assert!(append_job("", entry).is_err(), "accepted {entry}");
        }
        assert!(append_job("*/15 * * * * /same\n", "*/15 * * * * /same").is_err());
    }

    #[test]
    fn decodes_root_reads_and_authorization_failures() {
        use std::os::unix::process::ExitStatusExt;
        let output = |code, stdout: &str, stderr: &str| std::process::Output {
            status: std::process::ExitStatus::from_raw(code << 8),
            stdout: stdout.as_bytes().to_vec(),
            stderr: stderr.as_bytes().to_vec(),
        };
        assert_eq!(
            super::root_read_result(output(0, "# Keep\n0 * * * * /job\n\n", ""), true).unwrap(),
            "# Keep\n0 * * * * /job\n"
        );
        assert_eq!(
            super::root_read_result(output(0, "\n", ""), true).unwrap(),
            ""
        );
        assert!(super::root_read_result(
            output(1, "", "execution error: User canceled. (-128)"),
            true
        )
        .unwrap_err()
        .contains("cancelled"));
        assert_eq!(
            super::root_read_result(output(1, "", "no crontab for root"), false).unwrap(),
            ""
        );
        assert!(super::root_read_result(output(126, "", "dismissed"), false)
            .unwrap_err()
            .contains("cancelled"));
        assert!(
            super::root_read_result(output(127, "", "No authentication agent found"), false)
                .is_err()
        );
    }

    #[test]
    fn replaces_selected_job_in_place_and_preserves_other_lines() {
        let current = "# Comment\nSHELL=/bin/sh\n0 * * * * /old\r\n@reboot /keep\n";
        assert_eq!(
            replace_job(current, 2, "30 9 * * * /new").unwrap(),
            "# Comment\nSHELL=/bin/sh\n30 9 * * * /new\r\n@reboot /keep\n"
        );
        assert!(replace_job(current, 0, "0 * * * * /new").is_err());
        assert!(replace_job(current, 2, "60 * * * * /new").is_err());
        assert!(replace_job("0 * * * * /old\n30 9 * * * /new\n", 0, "30 9 * * * /new").is_err());
        assert_eq!(
            replace_job("0 * * * * /old", 0, "30 9 * * * /new").unwrap(),
            "30 9 * * * /new\n"
        );
    }

    #[test]
    fn deletes_only_selected_occurrence_and_preserves_other_bytes() {
        let current = "# Keep\r\nSHELL=/bin/sh\n\n0 * * * * /same\n0 * * * * /same\n@reboot /start";
        assert_eq!(
            remove_job(current, 3).unwrap(),
            "# Keep\r\nSHELL=/bin/sh\n\n0 * * * * /same\n@reboot /start"
        );
        assert_eq!(remove_job("0 * * * * /only\n", 0).unwrap(), "");
        assert_eq!(remove_job("# Keep\n@reboot /start", 1).unwrap(), "# Keep\n");
        for index in [0, 1, 2, 99] {
            assert!(remove_job(current, index).is_err());
        }
    }

    #[test]
    fn preserves_existing_settings_comments_and_jobs() {
        let current = "SHELL=/bin/sh\n# My backup\n0 3 * * * /backup";
        assert_eq!(
            append_job(current, "30 9 * * 1 /usr/bin/true").unwrap(),
            format!("{current}\n30 9 * * 1 /usr/bin/true\n")
        );
        assert_eq!(
            append_job("", "0 * * * * /usr/bin/true").unwrap(),
            "0 * * * * /usr/bin/true\n"
        );
    }

    #[test]
    fn rejects_duplicate_and_invalid_jobs() {
        assert!(append_job("0 * * * * /usr/bin/true\n", "0 * * * * /usr/bin/true").is_err());
        for entry in [
            "60 * * * * /bad",
            "0 24 * * * /bad",
            "0 9 * * 7 /bad",
            "0 * * * * ",
            "0 * * * * /ok\n@reboot /bad",
        ] {
            assert!(append_job("", entry).is_err(), "accepted {entry}");
        }
    }
}
