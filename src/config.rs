use globset::{Glob, GlobSet, GlobSetBuilder};
use ssh2::Session;
use std::env;
use std::fs::File;
use std::io;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;

use crate::manifest::get_last_backup_folder;
use crate::ssh::connect_ssh;
use crate::utils::{AppResult, log_error};

pub struct BackupConfig {
    session: Session,
    remote_path: String,
    local_backup_previous_folder: Option<PathBuf>,
    local_backup_new_folder: PathBuf,
    pre_cmd: Option<String>,
    post_cmd: Option<String>,
    use_sudo: bool,
    excludes: GlobSet,
}
impl BackupConfig {
    pub fn new() -> AppResult<Self> {
        let local_backup_repo = required_env("BACKUP_LOCAL_REPO")?;
        let local_backup_repo_path = PathBuf::from(local_backup_repo);

        if !local_backup_repo_path.exists() {
            std::fs::create_dir_all(&local_backup_repo_path)?;
        }

        let local_backup_previous_folder = get_last_backup_folder(&local_backup_repo_path)?;
        let local_backup_new_folder =
            local_backup_repo_path.join(chrono::Utc::now().format("%Y%m%d%H%M%S").to_string());

        let server_ip = required_env("BACKUP_SERVER_IP")?;
        let ssh_port = env::var("BACKUP_SSH_PORT").unwrap_or_else(|_| "22".to_string());
        let ssh_user = required_env("BACKUP_SSH_USER")?;
        let private_key_path = PathBuf::from(required_env("BACKUP_PRIVATE_KEY_PATH")?);
        let private_key_passphrase = env::var("BACKUP_PRIVATE_KEY_PASSPHRASE").ok();
        let remote_path = required_env("BACKUP_REMOTE_PATH")?;
        let pre_cmd = env::var("BACKUP_PRE_CMD").ok();
        let post_cmd = env::var("BACKUP_POST_CMD").ok();
        let use_sudo = env::var("BACKUP_USE_SUDO")
            .map(|v| matches!(v.to_lowercase().as_str(), "1" | "true" | "yes"))
            .unwrap_or(false);

        let session = match connect_ssh(
            server_ip.as_str(),
            ssh_port.as_str(),
            ssh_user.as_str(),
            private_key_path.as_path(),
            private_key_passphrase.as_deref(),
        ) {
            Ok(sess) => sess,
            Err(e) => {
                log_error(&format!("Failed to establish SSH connection: {}", e));
                return Err(e);
            }
        };

        let mut globset_builder = GlobSetBuilder::new();
        if let Ok(exclude_path) = env::var("BACKUP_EXCLUDE_FILE") {
            if let Ok(file) = File::open(&exclude_path) {
                let reader = BufReader::new(file);
                for line in reader.lines().map_while(Result::ok) {
                    let line = line.trim();
                    if !line.is_empty() && !line.starts_with('#') {
                        if let Ok(glob) = Glob::new(line) {
                            globset_builder.add(glob);
                        }
                    }
                }
            } else {
                log_error(&format!(
                    "Failed to open exclude file at '{}'. No patterns will be excluded.",
                    exclude_path
                ));
            }
        }
        let excludes = globset_builder
            .build()
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?;

        Ok(BackupConfig {
            session,
            remote_path,
            local_backup_previous_folder,
            local_backup_new_folder,
            pre_cmd,
            post_cmd,
            use_sudo,
            excludes,
        })
    }

    pub fn session(&self) -> &Session {
        &self.session
    }
    pub fn remote_path(&self) -> &str {
        &self.remote_path
    }
    pub fn local_backup_previous_folder(&self) -> Option<&PathBuf> {
        self.local_backup_previous_folder.as_ref()
    }
    pub fn local_backup_new_folder(&self) -> &PathBuf {
        &self.local_backup_new_folder
    }
    pub fn pre_cmd(&self) -> Option<&String> {
        self.pre_cmd.as_ref()
    }
    pub fn post_cmd(&self) -> Option<&String> {
        self.post_cmd.as_ref()
    }
    pub fn use_sudo(&self) -> bool {
        self.use_sudo
    }
    pub fn excludes(&self) -> &GlobSet {
        &self.excludes
    }
}

fn required_env(key: &str) -> AppResult<String> {
    env::var(key).map_err(|_| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("Missing environment variable: {key}"),
        )
        .into()
    })
}
