use std::{
    io,
    process::{Command, ExitStatus, Stdio},
};

pub(crate) fn run(
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
    let url_path = url.split('/').last().unwrap_or("download");
    let file_path = PathBuf::from(url_path);

    if !file_path.exists() {
        run(&format!("curl -fSsL -O {}", url), true, None, false, 0)?;
    }

    let (status, stdout, _) = run(
        &format!("md5sum {}", file_path.display()),
        true,
        None,
        true,
        0,
    )?;
    let checksum = stdout.split_whitespace().next().unwrap_or("");

    if checksum != md5sum {
        return Err(io::Error::new(
            io::ErrorKind::Other,
            format!(
                "The installer file checksum does not match. Won't continue installation. \
                Try deleting {} and trying again.",
                file_path.display()
            ),
        ));
    }

    Ok(file_path)
}

pub(crate) fn get_kernel_version() -> io::Result<String> {
    let (_, stdout, _) = run("uname -r", true, None, false, 0)?;
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

    // Extract the version part (e.g., "5.15.0-1015" from "linux-image-5.15.0-1015-aws")
    let version_part = package
        .strip_prefix(prefix)
        .and_then(|s| s.strip_suffix(suffix))
        .or_else(|| {
            package.strip_prefix(prefix).and_then(|s| {
                let end_idx = s.find(suffix)?;
                Some(&s[..end_idx])
            })
        })?;

    // Split by "-" to get major.minor.patch and micro
    let parts: Vec<&str> = version_part.split('-').collect();
    if parts.len() != 2 {
        return None;
    }

    // Get the patch from major.minor.patch
    let version_numbers: Vec<&str> = parts[0].split('.').collect();
    if version_numbers.len() < 3 {
        return None;
    }

    let patch = version_numbers[2].parse::<usize>().ok()?;
    let micro = parts[1].parse::<usize>().ok()?;

    Some((patch, micro))
}

pub(crate) fn lock_kernel_updates_debian() -> io::Result<()> {
    println!("Locking kernel updates ...");

    let kernel_version = get_kernel_version()?;
    run(
        &format!(
            "apt-mark hold linux-image-{} linux-headers-{} linux-image-cloud-amd64 linux-headers-cloud-amd64",
            kernel_version, kernel_version
        ),
        true,
        None,
        false,
        0,
    )?;

    Ok(())
}

pub(crate) fn unlock_kernel_updates_debian() -> io::Result<()> {
    println!("Unlocking kernel updates...");

    let kernel_version = get_kernel_version()?;
    run(
        &format!(
            "apt-mark unhold linux-image-{} linux-headers-{} linux-image-cloud-amd64 linux-headers-cloud-amd64",
            kernel_version, kernel_version
        ),
        true,
        None,
        false,
        0,
    )?;

    Ok(())
}

pub(crate) fn reboot() -> ! {
    println!("The system needs to be rebooted to complete the installation process.");
    println!("The process will be continued after the reboot.");

    run("reboot now", true, None, false, 0).unwrap();
    std::process::exit(0);
}
