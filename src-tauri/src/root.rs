use std::io::{Read, Write};
use std::os::unix::fs::{DirBuilderExt, MetadataExt, OpenOptionsExt};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct ChangeRequest {
    expected: String,
    change: RootChange,
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum RootChange {
    Add { entry: String },
    Edit { line_index: usize, entry: String },
    Delete { line_index: usize },
}

fn apply_change(current: &str, change: &RootChange) -> Result<String, String> {
    match change {
        RootChange::Add { entry } => super::append_job(current, entry),
        RootChange::Edit { line_index, entry } => super::replace_job(current, *line_index, entry),
        RootChange::Delete { line_index } => super::remove_job(current, *line_index),
    }
}

const MAX_REQUEST: usize = 2 * 1024 * 1024;

// This branch runs before Tauri starts, so the elevated process opens no GUI.
pub fn run_helper_if_requested() -> bool {
    let args: Vec<_> = std::env::args_os().collect();
    if args.get(1).is_none_or(|arg| arg != "--root-change") {
        return false;
    }
    let result = if args.len() == 3 {
        change_as_root(Path::new(&args[2]))
    } else {
        Err("Invalid root helper arguments.".into())
    };
    match result {
        Ok(backup) => println!("{backup}"),
        Err(error) => {
            eprintln!("{error}");
            std::process::exit(1);
        }
    }
    true
}

fn parse_request(bytes: &[u8]) -> Result<ChangeRequest, String> {
    if bytes.len() > MAX_REQUEST {
        return Err("The root save request is too large.".into());
    }
    let request: ChangeRequest =
        serde_json::from_slice(bytes).map_err(|_| "Invalid root save request.".to_string())?;
    apply_change(&request.expected, &request.change)?;
    Ok(request)
}

fn change_as_root(path: &Path) -> Result<String, String> {
    // geteuid has no arguments or memory access requirements.
    if unsafe { libc::geteuid() } != 0 {
        return Err("The root helper requires system administrator authorization.".into());
    }
    let mut bytes = Vec::new();
    std::fs::File::open(path)
        .map_err(|e| format!("Could not open save request: {e}"))?
        .take((MAX_REQUEST + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|e| format!("Could not read save request: {e}"))?;
    let request = parse_request(&bytes)?;
    #[cfg(target_os = "macos")]
    let directory =
        Path::new("/var/root/Library/Application Support/com.jimkelley.rustycrone/backups");
    #[cfg(not(target_os = "macos"))]
    let directory = Path::new("/var/lib/rustycrone/backups");
    std::fs::DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(directory)
        .map_err(|e| format!("Could not create root backup folder: {e}"))?;
    let metadata = std::fs::symlink_metadata(directory).map_err(|e| e.to_string())?;
    if !metadata.is_dir() || metadata.uid() != 0 || metadata.mode() & 0o077 != 0 {
        return Err("Root backup folder must be a private directory owned by root.".into());
    }
    super::change_current_table(directory, "root", &request.expected, |current| {
        apply_change(current, &request.change)
    })
}

struct RequestDirectory(PathBuf);
impl Drop for RequestDirectory {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(self.0.join("request.json"));
        let _ = std::fs::remove_dir(&self.0);
    }
}

#[tauri::command]
pub async fn save_root_job(expected: String, entry: String) -> Result<String, String> {
    submit_root_change(expected, RootChange::Add { entry }).await
}

#[tauri::command]
pub async fn edit_root_job(
    expected: String,
    line_index: usize,
    entry: String,
) -> Result<String, String> {
    submit_root_change(expected, RootChange::Edit { line_index, entry }).await
}

#[tauri::command]
pub async fn delete_root_job(expected: String, line_index: usize) -> Result<String, String> {
    submit_root_change(expected, RootChange::Delete { line_index }).await
}

async fn submit_root_change(expected: String, change: RootChange) -> Result<String, String> {
    let bytes =
        serde_json::to_vec(&ChangeRequest { expected, change }).map_err(|e| e.to_string())?;
    parse_request(&bytes)?;
    tauri::async_runtime::spawn_blocking(move || {
        let _guard = super::SAVE_LOCK
            .try_lock()
            .map_err(|_| "Another save is in progress.".to_string())?;
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(|e| e.to_string())?
            .as_nanos();
        let directory =
            std::env::temp_dir().join(format!("rustycrone-{}-{timestamp}", std::process::id()));
        std::fs::DirBuilder::new()
            .mode(0o700)
            .create(&directory)
            .map_err(|e| format!("Could not prepare private save request: {e}"))?;
        let cleanup = RequestDirectory(directory);
        let path = cleanup.0.join("request.json");
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&path)
            .map_err(|e| e.to_string())?;
        file.write_all(&bytes)
            .and_then(|_| file.sync_all())
            .map_err(|e| e.to_string())?;
        drop(file);
        authorize_change(&std::env::current_exe().map_err(|e| e.to_string())?, &path)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[cfg(any(target_os = "macos", test))]
fn shell_quote(text: &str) -> String {
    format!("'{}'", text.replace('\'', "'\\''"))
}

#[cfg(target_os = "macos")]
fn authorize_change(executable: &Path, request: &Path) -> Result<String, String> {
    const SCRIPT: &str = "on run argv\nreturn do shell script (item 1 of argv) with administrator privileges without altering line endings\nend run";
    let command = format!(
        "{} --root-change {}",
        shell_quote(executable.to_str().ok_or("Invalid app path")?),
        shell_quote(request.to_str().ok_or("Invalid request path")?)
    );
    let output = Command::new("/usr/bin/osascript")
        .current_dir("/")
        .args(["-e", SCRIPT, &command])
        .stdin(Stdio::null())
        .output()
        .map_err(|e| format!("Could not open administrator authorization: {e}"))?;
    save_result(output, true)
}

#[cfg(target_os = "linux")]
fn authorize_change(executable: &Path, request: &Path) -> Result<String, String> {
    let output = Command::new("/usr/bin/pkexec").current_dir("/")
        .arg("--disable-internal-agent").arg(executable).arg("--root-change").arg(request)
        .stdin(Stdio::null()).output()
        .map_err(|e| format!("Could not open administrator authorization: {e}. Linux requires pkexec and a desktop PolicyKit agent."))?;
    save_result(output, false)
}

fn save_result(output: std::process::Output, macos: bool) -> Result<String, String> {
    if output.status.success() {
        let backup =
            String::from_utf8(output.stdout).map_err(|_| "Invalid helper response.".to_string())?;
        let backup = backup.trim_end_matches('\n');
        if backup.starts_with('/') && backup.ends_with(".crontab") {
            return Ok(backup.into());
        }
        return Err(
            "Could not confirm the root save. Reload root's crontab before retrying.".into(),
        );
    }
    let diagnostic = String::from_utf8_lossy(&output.stderr);
    if (macos && diagnostic.contains("(-128)")) || (!macos && output.status.code() == Some(126)) {
        return Err("Administrator authorization was cancelled. No changes were made.".into());
    }
    if macos
        && diagnostic.contains("crontab: tmp/tmp.")
        && diagnostic.contains("Operation not permitted")
    {
        let executable = std::env::current_exe()
            .map(|path| path.display().to_string())
            .unwrap_or_else(|_| "the RustyCrone executable".into());
        return Err(format!("macOS blocked access to cron's protected storage. This usually requires Full Disk Access, separately from administrator approval.\n\nIn System Settings > Privacy & Security > Full Disk Access, add RustyCrone, then quit and reopen it. For this development build, the executable is:\n{executable}\n\nReload root's crontab before trying again.\n\nOriginal error: {}", diagnostic.trim()));
    }
    Err(format!(
        "Could not save root's crontab: {}",
        diagnostic.trim()
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_request_without_executing_commands() {
        let request = ChangeRequest {
            expected: "# Keep\n".into(),
            change: RootChange::Add {
                entry: "0 * * * * /usr/bin/true".into(),
            },
        };
        assert!(parse_request(&serde_json::to_vec(&request).unwrap()).is_ok());
        assert!(parse_request(
            br#"{"expected":"","change":{"kind":"add","entry":"0 * * * * /ok\n@reboot /bad"}}"#
        )
        .is_err());
        assert!(parse_request(&vec![b'x'; MAX_REQUEST + 1]).is_err());
        assert!(parse_request(b"invalid").is_err());
    }

    #[test]
    fn quotes_shell_paths_as_literal_arguments() {
        assert_eq!(
            shell_quote("/app's folder/$(touch nope)"),
            "'/app'\\''s folder/$(touch nope)'"
        );
    }

    #[test]
    fn validates_root_edit_and_delete_requests_against_exact_lines() {
        let original = "# Keep\nSHELL=/bin/sh\n0 * * * * /same\n0 * * * * /same\n";
        for (change, expected_result) in [
            (
                RootChange::Edit {
                    line_index: 3,
                    entry: "30 9 * * * /new".into(),
                },
                "# Keep\nSHELL=/bin/sh\n0 * * * * /same\n30 9 * * * /new\n",
            ),
            (
                RootChange::Delete { line_index: 3 },
                "# Keep\nSHELL=/bin/sh\n0 * * * * /same\n",
            ),
        ] {
            let request = ChangeRequest {
                expected: original.into(),
                change,
            };
            let parsed = parse_request(&serde_json::to_vec(&request).unwrap()).unwrap();
            assert_eq!(
                apply_change(original, &parsed.change).unwrap(),
                expected_result
            );
        }
        for index in [0, 1, 99] {
            let request = ChangeRequest {
                expected: original.into(),
                change: RootChange::Delete { line_index: index },
            };
            assert!(parse_request(&serde_json::to_vec(&request).unwrap()).is_err());
        }
        assert!(
            parse_request(br#"{"expected":"","change":{"kind":"execute","entry":"/bad"}}"#)
                .is_err()
        );
        assert!(
            parse_request(br#"{"expected":"","change":{"kind":"delete","line_index":-1}}"#)
                .is_err()
        );
        assert_eq!(
            apply_change("0 * * * * /only\n", &RootChange::Delete { line_index: 0 }).unwrap(),
            ""
        );
    }

    #[test]
    fn explains_macos_cron_storage_denial_without_hiding_other_errors() {
        use std::os::unix::process::ExitStatusExt;
        let output = |message: &str| std::process::Output {
            status: std::process::ExitStatus::from_raw(1 << 8),
            stdout: Vec::new(),
            stderr: message.as_bytes().to_vec(),
        };
        let message = save_result(output("execution error: Crontab rejected the change: crontab: tmp/tmp.25924: Operation not permitted (1)"), true).unwrap_err();
        assert!(message.contains("Full Disk Access"));
        assert!(message.contains("Original error:"));
        assert!(!save_result(output("crontab: syntax error"), true)
            .unwrap_err()
            .contains("Full Disk Access"));
        assert!(!save_result(
            output("crontab: tmp/tmp.25924: Operation not permitted"),
            false
        )
        .unwrap_err()
        .contains("Full Disk Access"));
    }
}
