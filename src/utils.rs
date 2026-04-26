use std::error::Error;
use std::fs;
use std::path::Path;

pub type AppResult<T> = Result<T, Box<dyn Error>>;

pub fn log_info(message: &str) {
    println!("\x1b[32m[INFO]\x1b[0m {message}");
}

pub fn log_warn(message: &str) {
    eprintln!("\x1b[33m[WARN]\x1b[0m {message}");
}

pub fn log_error(message: &str) {
    eprintln!("\x1b[31m[ERROR]\x1b[0m {message}");
}

pub fn shell_escape_single_quotes(input: &str) -> String {
    input.replace('\'', "'\\''")
}

pub fn create_hardlink(old_backup_file: &Path, new_backup_file: &Path) -> AppResult<()> {
    if let Some(parent) = new_backup_file.parent() {
        if let Err(error) = std::fs::create_dir_all(parent) {
            log_error(&format!(
                "Failed to create directory for hard link {:?}: {}",
                new_backup_file, error
            ));
            return Err(error.into());
        }
    }

    match fs::hard_link(old_backup_file, new_backup_file) {
        Ok(_) => Ok(()),
        Err(e) => {
            log_error(&format!(
                "Failed to create hard link from {:?} to {:?}: {}",
                old_backup_file, new_backup_file, e
            ));
            Err(e.into())
        }
    }
}
