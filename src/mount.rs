use std::{
    env, fs, io,
    os::unix::fs::FileTypeExt,
    path::{Path, PathBuf},
};

use clap::{Args, ValueEnum};

use crate::utils::{run_cmd, CommandOptions};

const FSTAB_PATH: &str = "/etc/fstab";

#[derive(Debug, Clone, Args)]
pub(crate) struct MountCommand {
    /// Device name or full path, for example `nvme0n2` or `/dev/nvme0n2`
    pub(crate) device: String,

    /// Mountpoint path, for example `/mnt/disks/public`
    pub(crate) mountpoint: String,

    /// Format the whole device when it does not already contain a filesystem
    #[arg(long)]
    pub(crate) provision: bool,

    /// Filesystem to create when provisioning
    #[arg(long, value_enum, default_value_t = Filesystem::Ext4)]
    pub(crate) fs: Filesystem,

    /// Optional filesystem label to apply during provisioning
    #[arg(long)]
    pub(crate) label: Option<String>,

    /// Owner to apply to the mounted directory root
    #[arg(long)]
    pub(crate) owner: Option<String>,

    /// Group to apply to the mounted directory root
    #[arg(long)]
    pub(crate) group: Option<String>,

    /// Optional chmod mode to apply after mounting, for example `775`
    #[arg(long)]
    pub(crate) mode: Option<String>,

    /// Allow provisioning even when the device already has signatures
    #[arg(long)]
    pub(crate) force: bool,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub(crate) enum Filesystem {
    Ext4,
    Xfs,
}

impl std::fmt::Display for Filesystem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Filesystem::Ext4 => write!(f, "ext4"),
            Filesystem::Xfs => write!(f, "xfs"),
        }
    }
}

pub(crate) fn configure_mount(command: MountCommand) -> io::Result<()> {
    validate_argument("device", &command.device)?;
    validate_argument("mountpoint", &command.mountpoint)?;
    validate_optional_argument("label", command.label.as_deref())?;
    validate_optional_argument("owner", command.owner.as_deref())?;
    validate_optional_argument("group", command.group.as_deref())?;
    validate_optional_argument("mode", command.mode.as_deref())?;

    let device_path = resolve_device_path(&command.device)?;
    let mount_config = match read_mount_config(&device_path)? {
        Some(config) => config,
        None => {
            if !command.provision {
                return Err(io::Error::other(format!(
                    "{} does not contain a filesystem. Re-run with --provision to format it as {} and mount it.",
                    device_path.display(),
                    command.fs
                )));
            }

            provision_device(
                &device_path,
                command.fs,
                command.label.as_deref(),
                command.force,
            )?;

            read_mount_config(&device_path)?.ok_or_else(|| {
                io::Error::other(format!(
                    "Provisioning {} completed but no filesystem metadata could be read afterward.",
                    device_path.display()
                ))
            })?
        }
    };

    fs::create_dir_all(&command.mountpoint)?;
    update_fstab(
        &mount_config.uuid,
        &command.mountpoint,
        &mount_config.fs_type,
    )?;
    ensure_mounted(&device_path, &command.mountpoint, &mount_config.uuid)?;
    apply_post_mount_settings(
        &command.mountpoint,
        command.owner.as_deref(),
        command.group.as_deref(),
        command.mode.as_deref(),
    )?;

    println!(
        "Mounted {} at {} and persisted it in {}.",
        device_path.display(),
        command.mountpoint,
        FSTAB_PATH
    );
    Ok(())
}

struct MountConfig {
    uuid: String,
    fs_type: String,
}

fn validate_argument(label: &str, value: &str) -> io::Result<()> {
    if value.trim().is_empty() {
        return Err(io::Error::other(format!("{label} cannot be empty")));
    }

    if value.chars().any(char::is_whitespace) {
        return Err(io::Error::other(format!(
            "{label} cannot contain whitespace"
        )));
    }

    Ok(())
}

fn validate_optional_argument(label: &str, value: Option<&str>) -> io::Result<()> {
    if let Some(value) = value {
        validate_argument(label, value)?;
    }

    Ok(())
}

fn resolve_device_path(device_name: &str) -> io::Result<PathBuf> {
    let candidate = if device_name.starts_with("/dev/") {
        PathBuf::from(device_name)
    } else {
        Path::new("/dev").join(device_name)
    };

    let metadata = fs::metadata(&candidate).map_err(|err| {
        io::Error::new(
            err.kind(),
            format!("Could not access device {}: {}", candidate.display(), err),
        )
    })?;

    if !metadata.file_type().is_block_device() {
        return Err(io::Error::other(format!(
            "{} is not a block device",
            candidate.display()
        )));
    }

    Ok(candidate)
}

fn read_mount_config(device_path: &Path) -> io::Result<Option<MountConfig>> {
    let device = device_path
        .to_str()
        .ok_or_else(|| io::Error::other("Device path is not valid UTF-8"))?;
    let output = run_cmd(
        "blkid",
        ["-o", "export", device],
        CommandOptions {
            check: false,
            silent: true,
            ..Default::default()
        },
    )?;
    if !output.status.success() {
        return Ok(None);
    }

    let mut uuid = None;
    let mut fs_type = None;

    for line in output.stdout.lines() {
        if let Some(value) = line.strip_prefix("UUID=") {
            uuid = Some(value.to_string());
        } else if let Some(value) = line.strip_prefix("TYPE=") {
            fs_type = Some(value.to_string());
        }
    }

    match (uuid, fs_type) {
        (Some(uuid), Some(fs_type)) => Ok(Some(MountConfig { uuid, fs_type })),
        (None, None) => Ok(None),
        _ => Err(io::Error::other(format!(
            "{} needs a filesystem with a UUID before it can be persisted in {}",
            device_path.display(),
            FSTAB_PATH
        ))),
    }
}

fn provision_device(
    device_path: &Path,
    filesystem: Filesystem,
    label: Option<&str>,
    force: bool,
) -> io::Result<()> {
    if device_has_signatures(device_path)? && !force {
        return Err(io::Error::other(format!(
            "{} already has signatures. Re-run with --force to format the whole device.",
            device_path.display()
        )));
    }

    let device = device_path
        .to_str()
        .ok_or_else(|| io::Error::other("Device path is not valid UTF-8"))?;
    match filesystem {
        Filesystem::Ext4 => {
            let mut args = vec!["-F", "-m", "0"];
            if let Some(label) = label {
                args.push("-L");
                args.push(label);
            }
            args.push(device);
            run_cmd("mkfs.ext4", args, CommandOptions::default())?;
        }
        Filesystem::Xfs => {
            let mut args = vec!["-f"];
            if let Some(label) = label {
                args.push("-L");
                args.push(label);
            }
            args.push(device);
            run_cmd("mkfs.xfs", args, CommandOptions::default())?;
        }
    }

    Ok(())
}

fn device_has_signatures(device_path: &Path) -> io::Result<bool> {
    let device = device_path
        .to_str()
        .ok_or_else(|| io::Error::other("Device path is not valid UTF-8"))?;
    let output = run_cmd(
        "wipefs",
        ["-n", device],
        CommandOptions {
            check: false,
            silent: true,
            ..Default::default()
        },
    )?;
    Ok(!output.stdout.trim().is_empty())
}

fn update_fstab(uuid: &str, mountpoint: &str, fs_type: &str) -> io::Result<()> {
    let content = fs::read_to_string(FSTAB_PATH)?;
    let new_entry = format!(
        "UUID={} {} {} defaults,nofail 0 2",
        uuid, mountpoint, fs_type
    );
    let uuid_selector = format!("UUID={uuid}");

    let mut updated_lines = Vec::new();
    let mut replaced = false;

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            updated_lines.push(line.to_string());
            continue;
        }

        let fields: Vec<&str> = trimmed.split_whitespace().collect();
        let matches_existing =
            fields.len() >= 2 && (fields[0] == uuid_selector || fields[1] == mountpoint);

        if matches_existing {
            if !replaced {
                updated_lines.push(new_entry.clone());
                replaced = true;
            }
            continue;
        }

        updated_lines.push(line.to_string());
    }

    if !replaced {
        if !updated_lines.is_empty() && !updated_lines.last().unwrap().is_empty() {
            updated_lines.push(String::new());
        }
        updated_lines.push(new_entry);
    }

    let mut rendered = updated_lines.join("\n");
    if !rendered.ends_with('\n') {
        rendered.push('\n');
    }

    fs::write(FSTAB_PATH, rendered)?;
    Ok(())
}

fn ensure_mounted(device_path: &Path, mountpoint: &str, uuid: &str) -> io::Result<()> {
    let output = run_cmd(
        "findmnt",
        ["-n", "-o", "SOURCE", "--mountpoint", mountpoint],
        CommandOptions {
            check: false,
            silent: true,
            ..Default::default()
        },
    )?;
    if !output.status.success() {
        run_cmd("mount", [mountpoint], CommandOptions::default())?;
        return Ok(());
    }

    let current_source = output.stdout.trim();
    let device = device_path
        .to_str()
        .ok_or_else(|| io::Error::other("Device path is not valid UTF-8"))?;
    let uuid_source = format!("UUID={uuid}");

    if current_source == device || current_source == uuid_source {
        println!("{mountpoint} is already mounted from {current_source}.");
        return Ok(());
    }

    Err(io::Error::other(format!(
        "{} is already mounted from {}",
        mountpoint, current_source
    )))
}

fn apply_post_mount_settings(
    mountpoint: &str,
    owner: Option<&str>,
    group: Option<&str>,
    mode: Option<&str>,
) -> io::Result<()> {
    let (owner, group) = resolve_owner_group(owner, group);

    match (owner.as_deref(), group.as_deref()) {
        (Some(owner), Some(group)) => {
            let owner_group = format!("{owner}:{group}");
            run_cmd(
                "chown",
                [owner_group.as_str(), mountpoint],
                CommandOptions::default(),
            )?;
        }
        (Some(owner), None) => {
            run_cmd("chown", [owner, mountpoint], CommandOptions::default())?;
        }
        (None, Some(group)) => {
            run_cmd("chgrp", [group, mountpoint], CommandOptions::default())?;
        }
        (None, None) => {}
    }

    if let Some(mode) = mode {
        run_cmd("chmod", [mode, mountpoint], CommandOptions::default())?;
    }

    Ok(())
}

fn resolve_owner_group(
    owner: Option<&str>,
    group: Option<&str>,
) -> (Option<String>, Option<String>) {
    if owner.is_some() || group.is_some() {
        return (
            owner.map(std::string::ToString::to_string),
            group.map(std::string::ToString::to_string),
        );
    }

    let default_owner = env::var("SUDO_UID").ok();
    let default_group = env::var("SUDO_GID").ok();
    (default_owner, default_group)
}
