use std::net::TcpStream;
use std::io::{self, Read, Write};
use std::path::Path;
use ssh2::Session;
use indicatif::ProgressBar;

use crate::config::BackupConfig;
use crate::utils::{AppResult, log_info, log_warn, shell_escape_single_quotes};

pub fn connect_ssh(server_ip: &str, ssh_port: &str, ssh_user: &str, private_key_path: &Path, private_key_passphrase: Option<&str>) -> AppResult<Session> {
    let tcp = TcpStream::connect(format!("{}:{}", server_ip, ssh_port))?;
    
    let mut session = Session::new()?;
    session.set_tcp_stream(tcp);
    session.handshake()?;

    session.userauth_pubkey_file(
        ssh_user,
        None,
        private_key_path,
        private_key_passphrase,
    )?;

    if !session.authenticated() {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "SSH authentication failed. Check credentials and key permissions.",
        )
        .into());
    }
    log_info("SSH connection established and authenticated.");
    
    Ok(session)
}

pub fn sync_file(config: &BackupConfig, remote_path: &str, local_path: &Path, progress_bar: &ProgressBar) -> AppResult<()> {
    if config.use_sudo() {
        return sync_file_sudo(config, remote_path, local_path, progress_bar);
    }

    match config.session().scp_recv(Path::new(remote_path)) {
        Ok((mut remote_file, _)) => {
            let mut local_file = std::fs::File::create(local_path)?;
            let mut buffer = [0u8; 8192];
            loop {
                let read_bytes = remote_file.read(&mut buffer)?;
                if read_bytes == 0 {
                    break;
                }
                local_file.write_all(&buffer[..read_bytes])?;
                progress_bar.inc(read_bytes as u64);
            }
            Ok(())
        }
        Err(_) => {
            log_warn(&format!("scp_recv failed for '{}', retrying with sudo cat.", remote_path));
            sync_file_sudo(config, remote_path, local_path, progress_bar)
        }
    }
}

fn sync_file_sudo(config: &BackupConfig, remote_path: &str, local_path: &Path, progress_bar: &ProgressBar) -> AppResult<()> {
    let quoted_path = format!("'{}'", shell_escape_single_quotes(remote_path));
    let mut channel = config.session().channel_session()?;
    channel.exec(&format!("sudo cat {}", quoted_path))?;

    let mut local_file = std::fs::File::create(local_path)?;
    let mut buffer = [0u8; 8192];
    loop {
        let read_bytes = channel.read(&mut buffer)?;
        if read_bytes == 0 {
            break;
        }
        local_file.write_all(&buffer[..read_bytes])?;
        progress_bar.inc(read_bytes as u64);
    }
    channel.wait_close()?;

    let exit_status = channel.exit_status()?;
    if exit_status != 0 {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            format!("sudo cat {} failed with exit status {}", remote_path, exit_status),
        )
        .into());
    }

    Ok(())
}

pub fn execute_remote_command(session: &Session, command: &str) -> AppResult<String> {
    let mut channel = session.channel_session()?;
    channel.exec(command)?;

    let mut output = String::new();
    channel.read_to_string(&mut output)?;
    channel.wait_close()?;

    Ok(output)
}

pub fn execute_pre_command(config: &BackupConfig) -> AppResult<String> {
    let cmd = match config.pre_cmd() {
        Some(cmd) => cmd,
        None => return Ok(String::new()),
    };
    execute_remote_command(config.session(), cmd)
}

pub fn execute_post_command(config: &BackupConfig) -> AppResult<String> {
    let cmd = match config.post_cmd() {
        Some(cmd) => cmd,
        None => return Ok(String::new()),
    };
    execute_remote_command(config.session(), cmd)
}
