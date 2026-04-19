use std::{
    ffi::{OsStr, OsString},
    io::{self, Write},
    path::{Path, PathBuf},
    process::{Command, ExitStatus, Stdio},
};

#[derive(Clone, Copy, Debug)]
pub(crate) struct CommandOptions<'a> {
    pub(crate) check: bool,
    pub(crate) input: Option<&'a str>,
    pub(crate) silent: bool,
    pub(crate) retries: usize,
}

impl Default for CommandOptions<'_> {
    fn default() -> Self {
        Self {
            check: true,
            input: None,
            silent: false,
            retries: 0,
        }
    }
}

pub(crate) struct CommandOutput {
    pub(crate) status: ExitStatus,
    pub(crate) stdout: String,
    pub(crate) stderr: String,
}

pub(crate) fn run_cmd<I, S>(
    program: &str,
    args: I,
    options: CommandOptions<'_>,
) -> io::Result<CommandOutput>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let args: Vec<OsString> = args
        .into_iter()
        .map(|arg| arg.as_ref().to_os_string())
        .collect();

    if !options.silent {
        let rendered_command = std::iter::once(OsString::from(program))
            .chain(args.iter().cloned())
            .map(|arg| arg.to_string_lossy().into_owned())
            .collect::<Vec<_>>()
            .join(" ");
        println!("Executing {}", rendered_command);
    }

    let mut try_count = 0;

    loop {
        let mut cmd = Command::new(program);
        cmd.args(&args);

        if options.input.is_some() {
            cmd.stdin(Stdio::piped());
        }
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        let mut child = cmd.spawn()?;

        if let Some(input_str) = options.input {
            if let Some(stdin) = child.stdin.as_mut() {
                stdin.write_all(input_str.as_bytes())?;
            }
        }

        let output = child.wait_with_output()?;
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        if !options.silent {
            if !stdout.is_empty() {
                println!("{}", stdout);
            }
            if !stderr.is_empty() {
                eprintln!("{}", stderr);
            }
        }

        if output.status.success() || try_count >= options.retries {
            if options.check && !output.status.success() {
                return Err(io::Error::other("Command exited with nonzero code"));
            }
            return Ok(CommandOutput {
                status: output.status,
                stdout,
                stderr,
            });
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
    run_cmd(
        "curl",
        ["-fsSL", "-o", &dest_path, url],
        CommandOptions::default(),
    )?;

    let output = run_cmd(
        "md5sum",
        [&dest_path],
        CommandOptions {
            silent: true,
            ..Default::default()
        },
    )?;
    let checksum = output.stdout.split_whitespace().next().unwrap_or("");

    if checksum != md5sum {
        return Err(io::Error::other(format!(
            "The installer file checksum does not match. Won't continue installation. \
                Try deleting {dest_path} and trying again.",
        )));
    }

    Ok(dest_path.into())
}

pub(crate) fn get_kernel_version() -> io::Result<String> {
    let output = run_cmd("uname", ["-r"], CommandOptions::default())?;
    Ok(output.stdout.trim().to_string())
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
    let image_package = format!("linux-image-{kernel_version}");
    let headers_package = format!("linux-headers-{kernel_version}");
    run_cmd(
        "apt-mark",
        ["hold", image_package.as_str(), headers_package.as_str()],
        CommandOptions::default(),
    )?;

    Ok(())
}

pub(crate) fn unlock_kernel_updates_debian() -> io::Result<()> {
    println!("Unlocking kernel updates...");

    let kernel_version = get_kernel_version()?;
    let image_package = format!("linux-image-{kernel_version}");
    let headers_package = format!("linux-headers-{kernel_version}");
    run_cmd(
        "apt-mark",
        ["unhold", image_package.as_str(), headers_package.as_str()],
        CommandOptions::default(),
    )?;

    Ok(())
}

pub(crate) fn reboot() -> ! {
    println!("The system needs to be rebooted to complete the installation process.");
    println!("The process will be continued after the reboot.");

    run_cmd("reboot", ["now"], CommandOptions::default()).unwrap();
    std::process::exit(0);
}

pub(crate) fn get_distro_id() -> io::Result<String> {
    let content = std::fs::read_to_string("/etc/os-release")?;
    for line in content.lines() {
        if let Some(id) = line.strip_prefix("ID=") {
            return Ok(id.trim_matches('"').to_string());
        }
    }
    Err(io::Error::other(
        "Could not determine distro from /etc/os-release",
    ))
}
