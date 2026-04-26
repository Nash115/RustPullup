use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use walkdir::WalkDir;

use crate::config::BackupConfig;
use crate::ssh::execute_remote_command;
use crate::utils::{AppResult, log_info, log_warn, shell_escape_single_quotes};

#[derive(Debug)]
pub struct FileMeta {
    size: u64,
    modified_time: u64,
}
impl FileMeta {
    pub fn new(size: u64, modified_time: u64) -> Self {
        Self {
            size,
            modified_time,
        }
    }

    pub fn size(&self) -> u64 {
        self.size
    }
}

pub struct SyncStatus {
    to_pull: Vec<String>,
    to_hardlink: Vec<String>,
}
impl SyncStatus {
    pub fn get(
        remote_manifest: &HashMap<String, FileMeta>,
        local_manifest: &HashMap<String, FileMeta>,
    ) -> SyncStatus {
        let mut to_pull = Vec::new();
        let mut to_hardlink = Vec::new();

        for (path, remote_meta) in remote_manifest {
            match local_manifest.get(path) {
                Some(local_meta) => {
                    if local_meta.size != remote_meta.size
                        || local_meta.modified_time < remote_meta.modified_time
                    {
                        to_pull.push(path.clone());
                    } else {
                        to_hardlink.push(path.clone());
                    }
                }
                None => to_pull.push(path.clone()),
            }
        }

        SyncStatus {
            to_pull,
            to_hardlink,
        }
    }

    pub fn to_pull(&self) -> &Vec<String> {
        &self.to_pull
    }
    pub fn to_hardlink(&self) -> &Vec<String> {
        &self.to_hardlink
    }
}

pub fn get_last_backup_folder(path: &Path) -> AppResult<Option<PathBuf>> {
    let mut dirs = Vec::new();

    for entry in WalkDir::new(path).min_depth(1).max_depth(1) {
        let entry = entry?;
        if entry.path().is_dir() {
            dirs.push(entry.into_path());
        }
    }

    dirs.sort();
    Ok(dirs.last().cloned())
}

fn parse_manifest(manifest: &str) -> HashMap<String, FileMeta> {
    let mut files = HashMap::new();
    for line in manifest.lines() {
        let parts: Vec<&str> = line.split('|').collect();
        if parts.len() == 3 {
            let path = parts[0].to_string();
            let size = match parts[1].parse::<u64>() {
                Ok(value) => value,
                Err(_) => {
                    log_warn(&format!("Skipping malformed manifest line (size): {line}"));
                    continue;
                }
            };
            let modified_time = match parts[2].parse::<u64>() {
                Ok(value) => value,
                Err(_) => {
                    log_warn(&format!("Skipping malformed manifest line (mtime): {line}"));
                    continue;
                }
            };
            files.insert(path, FileMeta::new(size, modified_time));
        }
    }
    files
}

pub fn get_remote_manifest(config: &BackupConfig) -> AppResult<HashMap<String, FileMeta>> {
    let escaped_remote_dir = shell_escape_single_quotes(config.remote_path());

    let sudo = if config.use_sudo() { "sudo " } else { "" };

    let command = format!(
        "{}find '{}' -type f -printf '%p|%s|%Ts\\n'",
        sudo, escaped_remote_dir
    );

    let output = execute_remote_command(config.session(), &command)?;

    let parsed_manifest = parse_manifest(&output);

    log_info(&format!(
        "Remote manifest built: {} files found.",
        parsed_manifest.len()
    ));

    Ok(parsed_manifest)
}

pub fn build_local_manifest(config: &BackupConfig) -> AppResult<HashMap<String, FileMeta>> {
    let mut manifest = HashMap::new();

    let backup_dir = match config.local_backup_previous_folder() {
        Some(dir) => dir,
        None => return Ok(manifest),
    };

    if !backup_dir.exists() {
        return Ok(manifest);
    }

    for entry in WalkDir::new(backup_dir) {
        let entry = match entry {
            Ok(value) => value,
            Err(error) => {
                log_warn(&format!("Skipping unreadable local entry: {error}"));
                continue;
            }
        };

        let path = entry.path();
        let manifest_path = match entry.path().strip_prefix(backup_dir) {
            Ok(value) => {
                let mut p = PathBuf::from("/");
                p.push(value);
                p
            }
            Err(error) => {
                log_warn(&format!("Skipping entry with invalid path: {error}"));
                continue;
            }
        };

        if path.is_file() {
            if let Ok(metadata) = entry.metadata() {
                let size = metadata.len();
                let mtime = metadata
                    .modified()
                    .unwrap_or(SystemTime::now())
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs();
                let key = manifest_path.to_string_lossy().to_string();
                manifest.insert(key, FileMeta::new(size, mtime));
            }
        }
    }

    log_info(&format!(
        "Local manifest built: {} files found.",
        manifest.len()
    ));
    Ok(manifest)
}
