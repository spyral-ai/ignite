use std::io;

use clap::{Parser, Subcommand, ValueEnum};

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

    let home_dir = args.home_dir.unwrap_or("/home/ubuntu".to_string());

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
        AppCommand::Clang { version } => install_clang::install_clang(version.as_usize())?,
        AppCommand::Nvim => install_nvim::install_nvim(home_dir)?,
        AppCommand::Rust => install_rust::install_rust(home_dir)?,
        AppCommand::InstallAll {
            cuda_version,
            clang_version,
        } => {
            println!("Installing all components...");

            // Install Rust first
            install_rust::install_rust(home_dir.clone())?;

            // This will install the driver first.
            install_cuda::install_cuda(args.cloud_provider, cuda_version)?;

            // Install Clang and Neovim
            install_clang::install_clang(clang_version.as_usize())?;
            //install_nvim::install_nvim(home_dir)?;

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

/// Clang versions that are supported. We use an enum here so that the user doesn't specify an
/// arbitrary number,
#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum ClangVersion {
    V15,
    V16,
    V17,
}

impl ClangVersion {
    pub fn as_usize(&self) -> usize {
        match self {
            ClangVersion::V15 => 15,
            ClangVersion::V16 => 16,
            ClangVersion::V17 => 17,
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

    /// The user home dir. If not specified, this will default
    /// to `/home/ubuntu'
    #[arg(short, long)]
    home_dir: Option<String>,
}

#[derive(Debug, Subcommand)]
enum AppCommand {
    /// CUDA installation commands
    #[command(subcommand)]
    Cuda(CudaCommand),

    /// Install Clang
    Clang {
        /// Clang version to install
        #[arg(short, long, value_enum, default_value = "v17")]
        version: ClangVersion,
    },

    /// Install Neovim
    Nvim,

    /// Install Rust
    Rust,

    /// Install all components (Rust, CUDA, Clang, Neovim)
    InstallAll {
        /// CUDA version to install
        #[arg(short, long, value_enum, default_value = "v13-0-1")]
        cuda_version: CudaVersion,

        /// Clang version to install
        #[arg(long, value_enum, default_value = "v17")]
        clang_version: ClangVersion,
    },
}

#[derive(Debug, Subcommand)]
enum CudaCommand {
    /// Install NVIDIA GPU driver
    InstallDriver {
        /// CUDA version to install
        #[arg(short, long, value_enum, default_value = "v13-0-1")]
        version: CudaVersion,
    },

    /// Install CUDA toolkit
    InstallCuda {
        /// CUDA version to install
        #[arg(short, long, value_enum, default_value = "v13-0-1")]
        version: CudaVersion,
    },

    /// Uninstall NVIDIA GPU driver
    UninstallDriver {
        /// CUDA version to uninstall
        #[arg(short, long, value_enum, default_value = "v13-0-1")]
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
