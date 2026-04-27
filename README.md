# RustPullup

Backup tool developed in Rust using SSH and SFTP

# Usage

1. Copy the `.env.example` file to `.env` and fill in the required environment variables.
2. Run the application using `cargo run` or build it using `cargo build --release` and execute the binary.

# Configuration

The application is configured using environment variables. Below are the required and optional variables:

| Varaible name                   | Description                                             | Required | Default |
| ------------------------------- | ------------------------------------------------------- | -------- | ------- |
| `BACKUP_SERVER_IP`              | The IP address of the server to be backed up            | YES      | N/A     |
| `BACKUP_SSH_PORT`               | The SSH port of the server to be backed up              | NO       | `22`    |
| `BACKUP_SSH_USER`               | The SSH user of the server to be backed up              | YES      | N/A     |
| `BACKUP_PRIVATE_KEY_PATH`       | The path to the SSH private key file                    | YES      | N/A     |
| `BACKUP_PRIVATE_KEY_PASSPHRASE` | The passphrase for the SSH private key                  | NO       | N/A     |
| `BACKUP_REMOTE_PATH`            | The path on the remote server to be backed up           | YES      | N/A     |
| `BACKUP_LOCAL_REPO`             | The local path where the backup will be stored          | YES      | N/A     |
| `BACKUP_EXCLUDE_FILE`           | Path of the file containing path to exclude from backup | NO       | N/A     |
| `BACKUP_PRE_CMD`                | The command to run before backing up                    | NO       | N/A     |
| `BACKUP_POST_CMD`               | The command to run after backing up                     | NO       | N/A     |
| `BACKUP_USE_SUDO`               | Whether to use sudo for pulling files                   | YES      | `false` |

# Exclude files (`backup.exclude`)

It is highly recommended to use an exclude file to prevent backing up unnecessary files and directories. The exclude file should contain one path per line. It is also recommended to use absolute paths.

Here is an example of an exclude file:

```
**/*.log
**/cache
**/.env
/home/ubuntu/my_secret_folder/*
```
