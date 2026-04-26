use indicatif::{ProgressBar, ProgressStyle};

use crate::config::BackupConfig;
use crate::manifest::{SyncStatus, build_local_manifest, get_remote_manifest};
use crate::ssh::sync_file;
use crate::utils::{AppResult, create_hardlink, log_error, log_info};

pub fn start_backup(config: &BackupConfig) -> AppResult<()> {
    let remote_manifest = get_remote_manifest(config)?;
    let local_manifest = build_local_manifest(config)?;

    let sync_status = SyncStatus::get(&remote_manifest, &local_manifest);
    let files_to_pull = sync_status.to_pull();
    let files_to_hardlink = sync_status.to_hardlink();

    // Pull new/updated files

    log_info(&format!("Pulling {} files.", files_to_pull.len()));

    let total_bytes_pull: u64 = files_to_pull
        .iter()
        .map(|path| {
            remote_manifest
                .get(path)
                .map(|meta| meta.size())
                .unwrap_or(0)
        })
        .sum();

    let progress_bar_pull = ProgressBar::new(total_bytes_pull);
    progress_bar_pull.set_style(
        ProgressStyle::with_template("[{elapsed_precise}] [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({bytes_per_sec}, ETA: {eta}) {msg}")?
            .progress_chars("#>-"),
    );

    std::fs::create_dir_all(config.local_backup_new_folder())?;

    for file in files_to_pull {
        // progress_bar_pull.set_message(file.clone());
        let local_path = config
            .local_backup_new_folder()
            .join(file.trim_start_matches('/'));

        if let Some(parent) = local_path.parent() {
            if let Err(error) = std::fs::create_dir_all(parent) {
                log_error(&format!(
                    "Failed to create local directory for {}: {}",
                    file, error
                ));
                continue;
            }
        }

        if let Err(error) = sync_file(config, &file, &local_path, &progress_bar_pull) {
            log_error(&format!("Failed to pull file {}: {}", file, error));
        }
    }
    progress_bar_pull.finish_with_message("Pull completed");

    // Create hardlinks for unchanged files

    log_info(&format!(
        "Creating hardlinks for {} unchanged files.",
        files_to_hardlink.len()
    ));

    let progress_bar_hardlink = ProgressBar::new(files_to_hardlink.len() as u64);
    progress_bar_hardlink.set_style(
        ProgressStyle::with_template(
            "[{elapsed_precise}] [{bar:40.green/blue}] {pos}/{len} ETA: {eta} {msg}",
        )?
        .progress_chars("#>-"),
    );

    for file in files_to_hardlink {
        progress_bar_hardlink.inc(1);
        progress_bar_hardlink.set_message(file.clone());

        let old_backup_file = config
            .local_backup_previous_folder()
            .unwrap()
            .join(file.trim_start_matches('/'));
        let new_backup_file = config
            .local_backup_new_folder()
            .join(file.trim_start_matches('/'));

        if let Err(error) = create_hardlink(&old_backup_file, &new_backup_file) {
            log_error(&format!(
                "Failed to create hard link for {}: {}",
                file, error
            ));
        }
    }
    progress_bar_hardlink.finish_with_message("Hardlink creation completed");

    Ok(())
}
