use std::{
    env,
    fs::{self, File},
    io::{self, Read, Write},
    path::{Path, PathBuf},
    process::{Command, ExitStatus, Stdio},
};

use clap::{Parser, Subcommand, ValueEnum};
use md5::{Digest, Md5};
use tempfile::TempDir;

pub(crate) mod install_clang;
pub(crate) mod install_cuda;
pub(crate) mod install_nvim;
pub(crate) mod install_rust;
pub(crate) mod utils;

use install_cuda::CudaVersion;

fn main() -> io::Result<()> {
    if !is_root() {
        eprintln!("This script needs to be run with root privileges!");
        std::process::exit(1);
    }

    let args = Args::parse();

    match args.command {
        AppCommand::Cuda(cmd) => match cmd {
            CudaCommand::InstallDriver { version } => {
                install_cuda::install_driver(args.cloud_provider, version)?
            }
            CudaCommand::InstallCuda { version } => {
                install_cuda::install_cuda(args.cloud_provider, version)?
            }
            CudaCommand::UninstallDriver { version } => install_cuda::uninstall_driver(version)?,
            CudaCommand::VerifyDriver => {
                if install_cuda::verify_driver(true)? {
                    std::process::exit(0);
                } else {
                    std::process::exit(1);
                }
            }
        },
        AppCommand::Clang => install_clang::install_clang()?,
        AppCommand::Nvim => install_nvim::install_nvim()?,
        AppCommand::Rust => install_rust::install_rust()?,
        AppCommand::InstallAll { cuda_version } => {
            println!("Installing all components...");

            // Install Rust first
            install_rust::install_rust()?;

            // Install CUDA driver and toolkit
            install_cuda::install_driver(args.cloud_provider, cuda_version)?;
            // Note: After driver installation, a reboot is typically required
            // The script will exit after reboot, so the following commands won't run until
            // the script is run again after reboot
            install_cuda::install_cuda(args.cloud_provider, cuda_version)?;

            // Install Clang and Neovim
            install_clang::install_clang()?;
            install_nvim::install_nvim()?;

            println!("All components installed successfully!");
        }
    }

    Ok(())
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum CloudProvider {
    Aws,
    Gcp,
    Azure,
}

impl CloudProvider {
    pub fn kernel_suffix(&self) -> &'static str {
        match self {
            CloudProvider::Aws => "-aws",
            CloudProvider::Gcp => "-cloud-amd64",
            CloudProvider::Azure => "-azure",
        }
    }
}

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Command to execute
    #[command(subcommand)]
    command: AppCommand,

    /// Cloud provider
    #[arg(short, long, value_enum, default_value = "gcp")]
    cloud_provider: CloudProvider,
}

#[derive(Debug, Subcommand)]
enum AppCommand {
    /// CUDA installation commands
    #[command(subcommand)]
    Cuda(CudaCommand),

    /// Install Clang
    Clang,

    /// Install Neovim
    Nvim,

    /// Install Rust
    Rust,

    /// Install all components (Rust, CUDA, Clang, Neovim)
    InstallAll {
        /// CUDA version to install
        #[arg(short, long, value_enum, default_value = "v12_8")]
        cuda_version: CudaVersion,
    },
}

#[derive(Debug, Subcommand)]
enum CudaCommand {
    /// Install NVIDIA GPU driver
    InstallDriver {
        /// CUDA version to install
        #[arg(short, long, value_enum, default_value = "v12_8")]
        version: CudaVersion,
    },

    /// Install CUDA toolkit
    InstallCuda {
        /// CUDA version to install
        #[arg(short, long, value_enum, default_value = "v12_8")]
        version: CudaVersion,
    },

    /// Uninstall NVIDIA GPU driver
    UninstallDriver {
        /// CUDA version to uninstall
        #[arg(short, long, value_enum, default_value = "v12_8")]
        version: CudaVersion,
    },

    /// Verify NVIDIA GPU driver installation
    VerifyDriver,
}

fn is_root() -> bool {
    #[cfg(unix)]
    {
        unsafe { libc::geteuid() == 0 }
    }
    #[cfg(not(unix))]
    {
        false
    }
}
