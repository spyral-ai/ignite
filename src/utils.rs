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
        if file_has_md5(file_path, md5sum)? {
            println!("File {dest_path} already exists and its checksum matches.");
            return Ok(dest_path.into());
        }

        println!("File {dest_path} has the wrong checksum; downloading it again.");
        std::fs::remove_file(file_path)?;
    }

    println!("Downloading {url} to {dest_path} ...");
    run_cmd(
        "curl",
        ["-fsSL", "-o", &dest_path, url],
        CommandOptions::default(),
    )?;

    if !file_has_md5(file_path, md5sum)? {
        return Err(io::Error::other(format!(
            "The installer file checksum does not match. Delete {dest_path} and try again."
        )));
    }

    Ok(dest_path.into())
}

fn file_has_md5(path: &Path, expected: &str) -> io::Result<bool> {
    let output = run_cmd(
        "md5sum",
        [path],
        CommandOptions {
            silent: true,
            ..Default::default()
        },
    )?;
    let checksum = output.stdout.split_whitespace().next().unwrap_or("");
    Ok(checksum == expected)
}
