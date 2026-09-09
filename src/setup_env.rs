use std::{
    fs,
    io::{self, ErrorKind},
    path::{Path, PathBuf},
};

use clap::Args;

use crate::{
    install_cuda::detect_cuda_home,
    utils::{run_cmd, CommandOptions},
};

const ENV_BLOCK_START: &str = "# >>> ignite setup-env >>>";
const ENV_BLOCK_END: &str = "# <<< ignite setup-env <<<";

#[derive(Debug, Clone, Args)]
pub(crate) struct SetupEnvCommand {
    /// Linux username whose ~/.bashrc should be updated
    #[arg(long)]
    pub(crate) user: String,

    /// Moonlite asset root, for example `/mnt/disks/public/moonlite`
    #[arg(long = "moonlite-dir")]
    pub(crate) moonlite_dir: String,

    /// Optional virtual environment root to activate from ~/.bashrc
    #[arg(long = "venv-dir")]
    pub(crate) venv_dir: Option<String>,
}

pub(crate) fn setup_env(command: SetupEnvCommand) -> io::Result<()> {
    validate_simple_arg("user", &command.user)?;
    validate_simple_arg("moonlite-dir", &command.moonlite_dir)?;

    let user_home = PathBuf::from(format!("/home/{}", command.user));
    if !user_home.is_dir() {
        return Err(io::Error::new(
            ErrorKind::NotFound,
            format!("{} does not exist", user_home.display()),
        ));
    }

    let moonlite_root = normalize_path(&command.moonlite_dir)?;
    let kernels_dir = moonlite_root.join("kernels");
    let models_dir = moonlite_root.join("models");
    fs::create_dir_all(&kernels_dir)?;
    fs::create_dir_all(&models_dir)?;

    let nccl_root = detect_nccl_root()?;
    let cuda_root = PathBuf::from(detect_cuda_home()?);
    let moonlite_kernels_src =
        user_home.join("Spyral/moonlite/crates/moonlite_compute/src/kernels");
    let venv_activate = command
        .venv_dir
        .as_deref()
        .map(normalize_path)
        .transpose()?
        .map(|venv_root| venv_root.join("bin/activate"))
        .map(|activate_path| {
            if activate_path.is_file() {
                Ok(activate_path)
            } else {
                Err(io::Error::new(
                    ErrorKind::NotFound,
                    format!(
                        "Expected virtual environment activation script at {}",
                        activate_path.display()
                    ),
                ))
            }
        })
        .transpose()?;
    let bashrc_path = user_home.join(".bashrc");
    let env_block = render_env_block(
        &nccl_root,
        &cuda_root,
        &models_dir,
        &moonlite_kernels_src,
        &kernels_dir,
        venv_activate.as_deref(),
    )?;

    upsert_bashrc_block(&bashrc_path, &env_block)?;

    let owner = format!("{}:{}", command.user, command.user);
    run_cmd(
        "chown",
        ["-R", owner.as_str(), path_arg(&kernels_dir)?],
        CommandOptions::default(),
    )?;
    run_cmd(
        "chown",
        ["-R", owner.as_str(), path_arg(&models_dir)?],
        CommandOptions::default(),
    )?;
    run_cmd(
        "chown",
        [owner.as_str(), path_arg(&bashrc_path)?],
        CommandOptions::default(),
    )?;

    println!("Configured environment for user {}.", command.user);
    println!("NCCL_ROOT={}", nccl_root.display());
    println!("CUDA_ROOT={}", cuda_root.display());
    println!("MOONLITE_MODELS_PATH={}", models_dir.display());
    println!("MOONLITE_KERNEL_OUTPUT_DIR={}", kernels_dir.display());

    Ok(())
}

fn validate_simple_arg(label: &str, value: &str) -> io::Result<()> {
    if value.trim().is_empty() {
        return Err(io::Error::other(format!("{label} cannot be empty")));
    }

    if value.contains('\n') {
        return Err(io::Error::other(format!("{label} cannot contain newlines")));
    }

    Ok(())
}

fn normalize_path(value: &str) -> io::Result<PathBuf> {
    let path = PathBuf::from(value);
    if path.is_absolute() {
        Ok(path)
    } else {
        Ok(std::env::current_dir()?.join(path))
    }
}

fn detect_nccl_root() -> io::Result<PathBuf> {
    for parent in [Path::new("/opt"), Path::new("/usr/local")] {
        let default_candidate = parent.join("nccl");
        if is_nccl_root(&default_candidate) {
            return Ok(default_candidate);
        }

        let mut candidates = Vec::new();
        if let Ok(entries) = fs::read_dir(parent) {
            for entry in entries {
                let entry = entry?;
                let path = entry.path();
                if !path.is_dir() {
                    continue;
                }

                if let Some(name) = path.file_name().and_then(|name| name.to_str()) {
                    if name.starts_with("nccl") && is_nccl_root(&path) {
                        candidates.push(path);
                    }
                }
            }
        }
        candidates.sort();

        if let Some(candidate) = candidates.into_iter().next() {
            return Ok(candidate);
        }
    }

    Err(io::Error::other(
        "Could not locate an NCCL installation under /opt or /usr/local",
    ))
}

fn is_nccl_root(path: &Path) -> bool {
    if !path.is_dir() {
        return false;
    }

    path.join("include/nccl.h").is_file() || contains_nccl_library(&path.join("lib"))
}

fn contains_nccl_library(lib_dir: &Path) -> bool {
    let Ok(entries) = fs::read_dir(lib_dir) else {
        return false;
    };

    entries
        .filter_map(Result::ok)
        .filter_map(|entry| entry.file_name().into_string().ok())
        .any(|name| name.starts_with("libnccl.so"))
}

fn render_env_block(
    nccl_root: &Path,
    cuda_root: &Path,
    models_dir: &Path,
    moonlite_kernels_src: &Path,
    kernels_dir: &Path,
    venv_activate: Option<&Path>,
) -> io::Result<String> {
    let source_venv = match venv_activate {
        Some(activate_path) => format!("source \"{}\"\n\n", path_arg(activate_path)?),
        None => String::new(),
    };

    Ok(format!(
        "{ENV_BLOCK_START}\n\
export NCCL_ROOT=\"{}\"\n\
export LD_LIBRARY_PATH=\"$NCCL_ROOT/lib${{LD_LIBRARY_PATH:+:$LD_LIBRARY_PATH}}\"\n\
export CPATH=\"$NCCL_ROOT/include${{CPATH:+:$CPATH}}\"\n\
\n\
export CUDA_ROOT=\"{}\"\n\
export PATH=\"$CUDA_ROOT/bin:$PATH\"\n\
export PATH=\"$HOME/.local/bin:$PATH\"\n\
\n\
{}\
export MOONLITE_PRINT_TENSOR_TRACE=1\n\
export MOONLITE_SNAPSHOT_FILTER=attention\n\
export MOONLITE_TRACE_FILTER=attention\n\
\n\
export MOONLITE_FORMAT_MAX_ELMENTS=4096\n\
export MOONLITE_FORMAT_EDGE_ITEMS=16,12\n\
\n\
export MOONLITE_LOG_MAX_MESSAGE_SIZE=64000\n\
export MOONLITE_MHA_QK_INTERLEAVE=0\n\
export MOONLITE_MODELS_PATH=\"{}\"\n\
\n\
export MOONLITE_KERNELS=\"{}\"\n\
export MOONLITE_KERNEL_OUTPUT_DIR=\"{}\"\n\
export MOONLITE_KERNEL_OPT_LEVEL=2\n\
{ENV_BLOCK_END}\n",
        path_arg(nccl_root)?,
        path_arg(cuda_root)?,
        source_venv,
        path_arg(models_dir)?,
        path_arg(moonlite_kernels_src)?,
        path_arg(kernels_dir)?,
    ))
}

fn upsert_bashrc_block(bashrc_path: &Path, env_block: &str) -> io::Result<()> {
    let mut content = if bashrc_path.exists() {
        fs::read_to_string(bashrc_path)?
    } else {
        String::new()
    };

    if let Some(start) = content.find(ENV_BLOCK_START) {
        if let Some(end_rel) = content[start..].find(ENV_BLOCK_END) {
            let end = start + end_rel + ENV_BLOCK_END.len();
            let mut updated = String::new();
            updated.push_str(&content[..start]);
            if !updated.is_empty() && !updated.ends_with('\n') {
                updated.push('\n');
            }
            updated.push_str(env_block);
            let suffix = content[end..].strip_prefix('\n').unwrap_or(&content[end..]);
            if !suffix.is_empty() && !suffix.starts_with('\n') {
                updated.push('\n');
            }
            updated.push_str(suffix);
            content = updated;
        } else {
            return Err(io::Error::other(format!(
                "{} contains {} without a matching {}",
                bashrc_path.display(),
                ENV_BLOCK_START,
                ENV_BLOCK_END
            )));
        }
    } else {
        if !content.is_empty() && !content.ends_with('\n') {
            content.push('\n');
        }
        if !content.is_empty() {
            content.push('\n');
        }
        content.push_str(env_block);
    }

    fs::write(bashrc_path, content)
}

fn path_arg(path: &Path) -> io::Result<&str> {
    path.to_str()
        .ok_or_else(|| io::Error::other(format!("{} is not valid UTF-8", path.display())))
}
