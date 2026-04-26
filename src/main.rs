mod backup;
mod config;
mod manifest;
mod ssh;
mod utils;

use crate::backup::start_backup;
use crate::config::BackupConfig;
use crate::ssh::{execute_post_command, execute_pre_command};
use crate::utils::{log_error, log_info};

fn main() {
    match dotenvy::dotenv() {
        Ok(_) => log_info("Environment variables loaded from .env."),
        Err(_) => {}
    };

    let config = match BackupConfig::new() {
        Ok(cfg) => cfg,
        Err(e) => {
            log_error(format!("Failed to load configuration: {}", e).as_str());
            std::process::exit(1);
        }
    };

    match execute_pre_command(&config) {
        Ok(output) => {
            if !output.trim().is_empty() {
                log_info(&format!("Pre-command output: {}", output.trim()));
            } else {
                log_info("Pre-command executed successfully with no output.");
            }
        }
        Err(e) => {
            log_error(format!("Failed to execute pre-command: {}", e).as_str());
            std::process::exit(1);
        }
    };

    match start_backup(&config) {
        Err(e) => {
            log_error(format!("Backup process failed: {}", e).as_str());
        }
        _ => {}
    };

    match execute_post_command(&config) {
        Ok(output) => {
            if !output.trim().is_empty() {
                log_info(&format!("Post-command output: {}", output.trim()));
            } else {
                log_info("Post-command executed successfully with no output.");
            }
        }
        Err(e) => {
            log_error(format!("Failed to execute post-command: {}", e).as_str());
            std::process::exit(1);
        }
    };
}
