use std::{
    io::{self, Write},
    path::{Path, PathBuf},
    process::{Command, ExitStatus, Stdio},
};

/// Runs the `command`, with no retries, and checks whether or not the command executed
/// successfuly.
///
/// This returns the status of the command, and the content of stdout and stderr, respectively.
///
/// For more control, use [`run_with_options`].
pub(crate) fn run(command: &str) -> io::Result<(ExitStatus, String, String)> {
    run_with_options(command, true, None, false, 0)
}

/// Runs the `command` with the provided options.
///
/// This returns the status of the command, and the content of stdout and stderr, respectively.
///
/// # Arguments
///
/// - `check` - Whether or not the output of the command is checked.
/// - `input` - Input to the command.
/// - `silent` - If the command output must be supressed.
/// - `retries` - The number of times to retry the command.
#[allow(unused_assignments)]
pub(crate) fn run_with_options(
    command: &str,
    check: bool,
    input: Option<&str>,
    silent: bool,
    retries: usize,
) -> io::Result<(ExitStatus, String, String)> {
    if !silent {
        println!("Executing {}", command);
    }

    let mut stdout_content = String::new();
    let mut stderr_content = String::new();
    let mut try_count = 0;

    loop {
        let mut parts = command.split_whitespace();
        let program = parts.next().unwrap();
        let args: Vec<&str> = parts.collect();

        let mut cmd = Command::new(program);
        cmd.args(&args);

        if input.is_some() {
            cmd.stdin(Stdio::piped());
        }
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        let mut child = cmd.spawn()?;

        if let Some(input_str) = input {
            if let Some(stdin) = child.stdin.as_mut() {
                stdin.write_all(input_str.as_bytes())?;
            }
        }

        let output = child.wait_with_output()?;

        stdout_content = String::from_utf8_lossy(&output.stdout).to_string();
        stderr_content = String::from_utf8_lossy(&output.stderr).to_string();

        if !silent {
            if !stdout_content.is_empty() {
                println!("{}", stdout_content);
            }
            if !stderr_content.is_empty() {
                eprintln!("{}", stderr_content);
            }
        }

        if output.status.success() || try_count >= retries {
            if check && !output.status.success() {
                return Err(io::Error::new(
                    io::ErrorKind::Other,
                    "Command exited with nonzero code",
                ));
            }
            return Ok((output.status, stdout_content, stderr_content));
        }

        try_count += 1;
    }
}

pub(crate) fn download_file(url: &str, md5sum: &str) -> io::Result<PathBuf> {
    let filename = url.split('/').next_back().unwrap_or("downloaded_file");
    let dest_path = format!("/tmp/{}", filename);

    let file_path = Path::new(&dest_path);
    if file_path.exists() {
        println!("File {dest_path} already exists, skipping download.");
        return Ok(dest_path.into());
    }

    println!("Downloading {url} to {dest_path} ...");
    run_with_options(
        &format!("curl -fsSL -o {dest_path} {url}"),
        true,
        None,
        false,
        0,
    )?;

    let (_status, stdout, _) =
        run_with_options(&format!("md5sum {dest_path}"), true, None, true, 0)?;
    let checksum = stdout.split_whitespace().next().unwrap_or("");

    if checksum != md5sum {
        return Err(io::Error::new(
            io::ErrorKind::Other,
            format!(
                "The installer file checksum does not match. Won't continue installation. \
                Try deleting {dest_path} and trying again.",
            ),
        ));
    }

    Ok(dest_path.into())
}

pub(crate) fn get_kernel_version() -> io::Result<String> {
    let (_, stdout, _) = run("uname -r")?;
    Ok(stdout.trim().to_string())
}

// Parse a package name like "linux-image-5.15.0-1015-aws" to extract version components
pub(crate) fn parse_kernel_package(
    package: &str,
    prefix: &str,
    suffix: &str,
) -> Option<(usize, usize)> {
    if !package.starts_with(prefix) || !package.contains(suffix) {
        return None;
    }

    // Extract the patch and micro (e.g., ".0-1015" from "linux-image-5.15.0-1015-aws")
    let version_part = package
        .strip_prefix(prefix)
        .and_then(|s| s.strip_suffix(suffix))
        .or_else(|| {
            package.strip_prefix(prefix).and_then(|s| {
                let end_idx = s.find(suffix)?;
                Some(&s[..end_idx])
            })
        })?;

    // Split by "-" to .patch and micro
    let parts: Vec<&str> = version_part.split('-').collect();
    if parts.len() != 2 {
        return None;
    }

    let patch = parts[0].strip_prefix(".").unwrap().parse::<usize>().ok()?;
    let micro = parts[1].parse::<usize>().ok()?;

    Some((patch, micro))
}

pub(crate) fn lock_kernel_updates_debian() -> io::Result<()> {
    println!("Locking kernel updates ...");

    let kernel_version = get_kernel_version()?;
    run(&format!(
        "apt-mark hold linux-image-{kernel_version} linux-headers-{kernel_version}",
    ))?;

    Ok(())
}

pub(crate) fn unlock_kernel_updates_debian() -> io::Result<()> {
    println!("Unlocking kernel updates...");

    let kernel_version = get_kernel_version()?;
    run(&format!(
        "apt-mark unhold linux-image-{kernel_version} linux-headers-{kernel_version}",
    ))?;

    Ok(())
}

pub(crate) fn reboot() -> ! {
    println!("The system needs to be rebooted to complete the installation process.");
    println!("The process will be continued after the reboot.");

    run("reboot now").unwrap();
    std::process::exit(0);
}

pub(crate) fn get_distro_id() -> io::Result<String> {
    let content = std::fs::read_to_string("/etc/os-release")?;
    for line in content.lines() {
        if let Some(id) = line.strip_prefix("ID=") {
            return Ok(id.trim_matches('"').to_string());
        }
    }
    Err(io::Error::new(
        io::ErrorKind::Other,
        "Could not determine distro from /etc/os-release",
    ))
}
