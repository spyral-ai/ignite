use std::{
    env,
    fs::{self, File},
    io::{self, Write},
    path::{Path, PathBuf},
};

use clap::{Args, ValueEnum};
use tempfile::TempDir;

use crate::{
    linux::{
        clear_resume_service, configure_resume_service, distro_id, distro_version,
        install_pinned_kernel, reboot, remove_kernel_pin, KernelConfig,
    },
    utils::*,
    CloudProvider,
};

const PROFILE_FILENAME: &str = "/etc/profile.d/spyral_cuda_install.sh";
const NCCL_PROFILE_FILENAME: &str = "/etc/profile.d/spyral_nccl.sh";
const DEFAULT_NCCL_INSTALL_DIR: &str = "/opt/nccl";
const NCCL_VERSION: &str = "2.30.3-1";
const NCCL_SOURCE_URL: &str = "https://github.com/NVIDIA/nccl/archive/refs/tags/v2.30.3-1.tar.gz";
const NVIDIA_PERSISTANCED_INSTALLER: &str =
    "/usr/share/doc/NVIDIA_GLX-1.0/samples/nvidia-persistenced-init.tar.bz2";

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, ValueEnum)]
pub enum CudaVersion {
    V12_5,
    V12_6,
    V12_8,
    V13_0_1,
    V13_1_1,
    V13_2_1,
    #[default]
    V13_3_1,
}

impl std::fmt::Display for CudaVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CudaVersion::V12_5 => write!(f, "12.5"),
            CudaVersion::V12_6 => write!(f, "12.6"),
            CudaVersion::V12_8 => write!(f, "12.8"),
            CudaVersion::V13_0_1 => write!(f, "13.0.1"),
            CudaVersion::V13_1_1 => write!(f, "13.1.1"),
            CudaVersion::V13_2_1 => write!(f, "13.2.1"),
            CudaVersion::V13_3_1 => write!(f, "13.3.1"),
        }
    }
}

impl CudaVersion {
    fn cli_value(self) -> &'static str {
        match self {
            Self::V12_5 => "v12-5",
            Self::V12_6 => "v12-6",
            Self::V12_8 => "v12-8",
            Self::V13_0_1 => "v13-0-1",
            Self::V13_1_1 => "v13-1-1",
            Self::V13_2_1 => "v13-2-1",
            Self::V13_3_1 => "v13-3-1",
        }
    }
}

#[derive(Clone, Copy)]
enum ResumeCommand {
    InstallDriver,
    InstallCuda,
}

impl ResumeCommand {
    fn cli_value(self) -> &'static str {
        match self {
            Self::InstallDriver => "install-driver",
            Self::InstallCuda => "install-cuda",
        }
    }
}

#[derive(Debug, Clone, Args)]
pub(crate) struct InstallNcclCommand {
    /// Installation directory for NCCL
    #[arg(long, default_value = DEFAULT_NCCL_INSTALL_DIR)]
    pub(crate) install_dir: String,

    /// Write /etc/profile.d/spyral_nccl.sh for system-wide NCCL environment variables
    #[arg(long)]
    pub(crate) write_profile: bool,
}

#[derive(Clone, Copy)]
enum KernelLine {
    V6_8,
    V6_17,
    V7_0,
}

impl KernelConfig {
    fn for_host(cloud_provider: CloudProvider, cuda_version: CudaVersion) -> io::Result<Self> {
        let distro_id = distro_id()?;
        let distro_version = distro_version()?;
        Self::for_platform(&distro_id, &distro_version, cloud_provider, cuda_version)
    }

    fn for_platform(
        distro_id: &str,
        distro_version: &str,
        cloud_provider: CloudProvider,
        cuda_version: CudaVersion,
    ) -> io::Result<Self> {
        if distro_id != "ubuntu" || distro_version != "24.04" {
            return Err(io::Error::other(format!(
                "Ignite has no pinned CUDA kernel for {distro_id} {distro_version} on \
                 {cloud_provider:?}. Pinned CUDA installs currently require Ubuntu 24.04."
            )));
        }

        let kernel_line = match cuda_version {
            CudaVersion::V12_5 | CudaVersion::V12_6 | CudaVersion::V12_8 => KernelLine::V6_8,
            CudaVersion::V13_0_1 | CudaVersion::V13_1_1 | CudaVersion::V13_2_1 => KernelLine::V6_17,
            CudaVersion::V13_3_1 => KernelLine::V7_0,
        };
        let (version, image_version, headers_version, modules_version, flavor) =
            match (cloud_provider, kernel_line) {
                (CloudProvider::Gcp, KernelLine::V6_8) => (
                    "6.8.0-1007-gcp",
                    "6.8.0-1007.7",
                    "6.8.0-1007.7",
                    "6.8.0-1007.7",
                    "gcp",
                ),
                (CloudProvider::Gcp, KernelLine::V6_17) => (
                    "6.17.0-1022-gcp",
                    "6.17.0-1022.25",
                    "6.17.0-1022.25",
                    "6.17.0-1022.25",
                    "gcp",
                ),
                (CloudProvider::Gcp, KernelLine::V7_0) => (
                    "7.0.0-1011-gcp",
                    "7.0.0-1011.11~24.04.1",
                    "7.0.0-1011.11~24.04.1",
                    "7.0.0-1011.11~24.04.1",
                    "gcp",
                ),
                (CloudProvider::Aws, KernelLine::V6_8) => (
                    "6.8.0-1008-aws",
                    "6.8.0-1008.8",
                    "6.8.0-1008.8",
                    "6.8.0-1008.8",
                    "aws",
                ),
                (CloudProvider::Aws, KernelLine::V6_17) => (
                    "6.17.0-1020-aws",
                    "6.17.0-1020.20~24.04.1+1",
                    "6.17.0-1020.20~24.04.1",
                    "6.17.0-1020.20~24.04.1",
                    "aws",
                ),
                (CloudProvider::Aws, KernelLine::V7_0) => (
                    "7.0.0-1012-aws",
                    "7.0.0-1012.12~24.04.1",
                    "7.0.0-1012.12~24.04.1",
                    "7.0.0-1012.12~24.04.1",
                    "aws",
                ),
                (CloudProvider::Azure, KernelLine::V6_8) => (
                    "6.8.0-1007-azure",
                    "6.8.0-1007.7",
                    "6.8.0-1007.7",
                    "6.8.0-1007.7",
                    "azure",
                ),
                (CloudProvider::Azure, KernelLine::V6_17) => (
                    "6.17.0-1022-azure",
                    "6.17.0-1022.22",
                    "6.17.0-1022.22",
                    "6.17.0-1022.22",
                    "azure",
                ),
                (CloudProvider::Azure, KernelLine::V7_0) => (
                    "7.0.0-1008-azure",
                    "7.0.0-1008.8~24.04.2",
                    "7.0.0-1008.8~24.04.2",
                    "7.0.0-1008.8~24.04.2",
                    "azure",
                ),
            };

        Ok(Self::new(
            version,
            flavor,
            image_version,
            headers_version,
            modules_version,
        ))
    }
}

struct CudaConfig {
    version: CudaVersion,
    toolkit_url: String,
    toolkit_checksum: String,
    bin_folder: String,
    lib_folder: String,
    driver_version: String,
}

impl CudaConfig {
    pub fn new(version: CudaVersion) -> Self {
        match version {
            CudaVersion::V12_5 => Self {
                version,
                toolkit_url: String::from(
                    "https://developer.download.nvidia.com/compute/cuda/12.5.0/local_installers/cuda_12.5.0_555.42.02_linux.run",
                ),
                toolkit_checksum: String::from("0bf587ce20c8e74b90701be56ae2c907"),
                bin_folder: String::from("/usr/local/cuda-12.5/bin"),
                lib_folder: String::from("/usr/local/cuda-12.5/lib64"),
                driver_version: String::from("555.42.02"),
            },
            CudaVersion::V12_6 => Self {
                version,
                toolkit_url: String::from(
                    "https://developer.download.nvidia.com/compute/cuda/12.6.0/local_installers/cuda_12.6.0_560.28.03_linux.run",
                ),
                toolkit_checksum: String::from("8685a58497b0c7e5d964e6da7968bb1e"),
                bin_folder: String::from("/usr/local/cuda-12.6/bin"),
                lib_folder: String::from("/usr/local/cuda-12.6/lib64"),
                driver_version: String::from("560.28.03"),
            },

            CudaVersion::V12_8 => Self {
                version,
                toolkit_url: String::from(
                    "https://developer.download.nvidia.com/compute/cuda/12.8.0/local_installers/cuda_12.8.0_570.86.10_linux.run",
                ),
                toolkit_checksum: String::from("c71027cf1a4ce84f80b9cbf81116e767"),
                bin_folder: String::from("/usr/local/cuda-12.8/bin"),
                lib_folder: String::from("/usr/local/cuda-12.8/lib64"),
                driver_version: String::from("570.86.10"),
            },
            CudaVersion::V13_0_1 => Self {
                version,
                toolkit_url: String::from(
                    "https://developer.download.nvidia.com/compute/cuda/13.0.1/local_installers/cuda_13.0.1_580.82.07_linux.run",
                ),
                toolkit_checksum: String::from("8c56e3cb1ab74370aafed5a4600bc5bc"),
                bin_folder: String::from("/usr/local/cuda-13.0/bin"),
                lib_folder: String::from("/usr/local/cuda-13.0/lib64"),
                driver_version: String::from("580.82.07"),
            },
            CudaVersion::V13_1_1 => Self {
                version,
                toolkit_url: String::from(
                    "https://developer.download.nvidia.com/compute/cuda/13.1.1/local_installers/cuda_13.1.1_590.48.01_linux.run",
                ),
                toolkit_checksum: String::from("8aa93a77cffa8d055db0ceb9d0e2d692"),
                bin_folder: String::from("/usr/local/cuda-13.1/bin"),
                lib_folder: String::from("/usr/local/cuda-13.1/lib64"),
                driver_version: String::from("590.48.01"),
            },
            CudaVersion::V13_2_1 => Self {
                version,
                toolkit_url: String::from(
                    "https://developer.download.nvidia.com/compute/cuda/13.2.1/local_installers/cuda_13.2.1_595.58.03_linux.run",
                ),
                toolkit_checksum: String::from("e5b4bdf19cc27d63a8254cb486764626"),
                bin_folder: String::from("/usr/local/cuda-13.2/bin"),
                lib_folder: String::from("/usr/local/cuda-13.2/lib64"),
                driver_version: String::from("595.58.03"),
            },
            CudaVersion::V13_3_1 => Self {
                version,
                toolkit_url: String::from(
                    "https://developer.download.nvidia.com/compute/cuda/13.3.1/local_installers/cuda_13.3.1_610.43.02_linux.run",
                ),
                toolkit_checksum: String::from("7c8d3eca60ee10d2c290bdc045f88f09"),
                bin_folder: String::from("/usr/local/cuda-13.3/bin"),
                lib_folder: String::from("/usr/local/cuda-13.3/lib64"),
                driver_version: String::from("610.43.02"),
            },
        }
    }
}

pub(crate) fn install_driver(
    cloud_provider: CloudProvider,
    cuda_version: CudaVersion,
) -> io::Result<()> {
    install_driver_inner(cloud_provider, cuda_version, ResumeCommand::InstallDriver)?;
    clear_resume_service()?;
    Ok(())
}

fn install_driver_inner(
    cloud_provider: CloudProvider,
    cuda_version: CudaVersion,
    resume_command: ResumeCommand,
) -> io::Result<()> {
    let cuda_config = CudaConfig::new(cuda_version);
    let kernel_config = KernelConfig::for_host(cloud_provider, cuda_version)?;
    let driver_is_ready = installed_driver_version()?.as_deref()
        == Some(cuda_config.driver_version.as_str())
        && verify_driver(false)?;

    if !driver_is_ready && Path::new("/usr/bin/nvidia-uninstall").exists() {
        println!("Removing the existing NVIDIA driver before configuring kernel packages...");
        run_cmd(
            "/usr/bin/nvidia-uninstall",
            ["--silent"],
            CommandOptions::default(),
        )?;
    }

    if install_pinned_kernel(&kernel_config)? {
        let cloud_provider = match cloud_provider {
            CloudProvider::Aws => "aws",
            CloudProvider::Gcp => "gcp",
            CloudProvider::Azure => "azure",
        };
        configure_resume_service(&[
            "--cloud-provider",
            cloud_provider,
            "cuda",
            resume_command.cli_value(),
            "--version",
            cuda_version.cli_value(),
        ])?;
        println!(
            "Rebooting into the pinned kernel {}. Ignite will continue the installation \
             automatically after the machine starts.",
            kernel_config.version
        );
        reboot();
    }

    if driver_is_ready {
        println!(
            "NVIDIA driver {} is already installed for kernel {}.",
            cuda_config.driver_version, kernel_config.version
        );
        return Ok(());
    }

    println!("Installing GPU drivers for CUDA {}...", cuda_version);

    let installer_path = download_cuda_toolkit_installer(&cuda_config)?;

    let installer = installer_path.to_string_lossy().into_owned();
    run_cmd(
        "sh",
        [installer.as_str(), "--silent", "--driver"],
        CommandOptions::default(),
    )?;

    if !verify_driver(true)? {
        return Err(io::Error::other(format!(
            "NVIDIA driver {} was installed but did not initialize on kernel {}",
            cuda_config.driver_version, kernel_config.version
        )));
    }

    println!("GPU driver installed successfully!");

    Ok(())
}

pub(crate) fn uninstall_driver(
    cloud_provider: CloudProvider,
    cuda_version: CudaVersion,
) -> io::Result<()> {
    let cuda_config = CudaConfig::new(cuda_version);

    if !verify_driver(false)? {
        println!("GPU driver not found.");
        return Ok(());
    }

    let temp_dir = TempDir::new()?;
    let installer_path = download_cuda_toolkit_installer(&cuda_config)?;

    println!("Extracting NVIDIA driver installer, to complete uninstallation...");
    let installer = installer_path.to_string_lossy().into_owned();
    let extract_arg = format!("--extract={}", temp_dir.path().display());
    run_cmd(
        "sh",
        [installer.as_str(), extract_arg.as_str()],
        CommandOptions::default(),
    )?;

    let installer_path = temp_dir.path().join(format!(
        "NVIDIA-Linux-x86_64-{}.run",
        cuda_config.driver_version
    ));

    println!("Starting uninstallation...");
    let installer = installer_path.to_string_lossy().into_owned();
    run_cmd(
        "sh",
        [installer.as_str(), "-s", "--uninstall"],
        CommandOptions::default(),
    )?;

    println!("Uninstallation completed!");
    remove_kernel_pin(&KernelConfig::for_host(cloud_provider, cuda_version)?)?;

    Ok(())
}

pub(crate) fn verify_driver(verbose: bool) -> io::Result<bool> {
    let output = run_cmd(
        "which",
        ["nvidia-smi"],
        CommandOptions {
            check: false,
            silent: true,
            ..Default::default()
        },
    )?;

    if !output.status.success() {
        if verbose {
            println!("Couldn't find nvidia-smi, the driver is not installed.");
        }
        return Ok(false);
    }

    let output = run_cmd(
        "nvidia-smi",
        ["-L"],
        CommandOptions {
            check: false,
            silent: true,
            ..Default::default()
        },
    )?;
    let success = output.status.success() && output.stdout.contains("UUID");

    if verbose {
        println!("nvidia-smi -L output: {} {}", output.stdout, output.stderr);
    }

    Ok(success)
}

fn installed_driver_version() -> io::Result<Option<String>> {
    let output = run_cmd(
        "nvidia-smi",
        ["--query-gpu=driver_version", "--format=csv,noheader"],
        CommandOptions {
            check: false,
            silent: true,
            ..Default::default()
        },
    )?;
    if !output.status.success() {
        return Ok(None);
    }

    Ok(output
        .stdout
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .map(str::to_string))
}

pub(crate) fn install_cuda(
    cloud_provider: CloudProvider,
    cuda_version: CudaVersion,
) -> io::Result<()> {
    install_cuda_inner(cloud_provider, cuda_version)?;
    clear_resume_service()?;
    Ok(())
}

pub(crate) fn install_nccl(command: InstallNcclCommand) -> io::Result<()> {
    if command.install_dir.trim().is_empty() {
        return Err(io::Error::other("install_dir cannot be empty"));
    }

    println!(
        "Installing NCCL {} to {}...",
        NCCL_VERSION, command.install_dir
    );

    let cuda_home = detect_cuda_home()?;
    run_cmd(
        "apt-get",
        ["install", "-y", "build-essential"],
        CommandOptions::default(),
    )?;

    let temp_dir = TempDir::new()?;
    let archive_path = temp_dir.path().join(format!("nccl-{NCCL_VERSION}.tar.gz"));
    let source_dir = temp_dir.path().join("src");
    let archive = archive_path.to_string_lossy().into_owned();
    let source = source_dir.to_string_lossy().into_owned();

    fs::create_dir_all(&source_dir)?;
    run_cmd(
        "curl",
        ["-fsSL", "-o", archive.as_str(), NCCL_SOURCE_URL],
        CommandOptions::default(),
    )?;
    run_cmd(
        "tar",
        [
            "-xzf",
            archive.as_str(),
            "-C",
            source.as_str(),
            "--strip-components=1",
        ],
        CommandOptions::default(),
    )?;

    let current_dir = env::current_dir()?;
    env::set_current_dir(&source_dir)?;
    let build_result = {
        let jobs = std::thread::available_parallelism()
            .map(|parallelism| parallelism.get())
            .unwrap_or(1)
            .to_string();
        let cuda_home_arg = format!("CUDA_HOME={cuda_home}");
        run_cmd(
            "make",
            ["-j", jobs.as_str(), "src.build", cuda_home_arg.as_str()],
            CommandOptions::default(),
        )
    };
    env::set_current_dir(current_dir)?;
    build_result?;

    install_built_nccl(&source_dir.join("build"), &command.install_dir)?;
    if command.write_profile {
        configure_nccl_environment(&command.install_dir)?;
    }
    verify_nccl_installation(&command.install_dir)?;

    println!(
        "NCCL {} installed successfully to {}.",
        NCCL_VERSION, command.install_dir
    );
    if command.write_profile {
        println!("Wrote {}", NCCL_PROFILE_FILENAME);
    }
    println!("Add the following to ~/.bashrc if you want NCCL on your default shell path:");
    for export in nccl_env_exports(&command.install_dir) {
        println!("{export}");
    }
    Ok(())
}

fn install_cuda_inner(cloud_provider: CloudProvider, cuda_version: CudaVersion) -> io::Result<()> {
    let cuda_config = CudaConfig::new(cuda_version);

    install_driver_inner(cloud_provider, cuda_version, ResumeCommand::InstallCuda)?;

    if Path::new(&format!("{}/nvcc", cuda_config.bin_folder)).exists() {
        println!(
            "Nvcc already installed at : {}/nvcc, not installing CUDA",
            cuda_config.bin_folder
        );
        return Ok(());
    }

    let installer_path = download_cuda_toolkit_installer(&cuda_config)?;

    println!("Installing CUDA {} toolkit...", cuda_version);
    let installer = installer_path.to_string_lossy().into_owned();
    run_cmd(
        "sh",
        [installer.as_str(), "--silent", "--toolkit"],
        CommandOptions::default(),
    )?;
    println!("CUDA toolkit installation completed!");

    println!("Executing post-installation actions...");
    cuda_postinstallation_actions(&cuda_config)?;
    println!("CUDA post-installation actions completed!");

    Ok(())
}

fn download_cuda_toolkit_installer(cuda_config: &CudaConfig) -> io::Result<PathBuf> {
    println!(
        "Downloading CUDA {} installation toolkit...",
        cuda_config.version
    );
    download_file(&cuda_config.toolkit_url, &cuda_config.toolkit_checksum)
}

fn configure_persistanced_service() -> io::Result<()> {
    if !Path::new("/usr/bin/nvidia-persistenced").exists() {
        return Ok(());
    }

    if !Path::new(NVIDIA_PERSISTANCED_INSTALLER).exists() {
        return Ok(());
    }

    let temp_dir = TempDir::new()?;
    fs::copy(
        NVIDIA_PERSISTANCED_INSTALLER,
        temp_dir.path().join("installer.tar.bz2"),
    )?;

    let current_dir = env::current_dir()?;
    env::set_current_dir(temp_dir.path())?;

    run_cmd(
        "tar",
        ["-xf", "installer.tar.bz2"],
        CommandOptions {
            silent: true,
            ..Default::default()
        },
    )?;
    println!("Executing nvidia-persistenced installer...");
    run_cmd(
        "sh",
        ["nvidia-persistenced-init/install.sh"],
        CommandOptions::default(),
    )?;

    env::set_current_dir(current_dir)?;
    Ok(())
}

fn install_built_nccl(build_dir: &Path, install_dir: &str) -> io::Result<()> {
    let include_dir = build_dir.join("include");
    let lib_dir = build_dir.join("lib");

    if !include_dir.exists() || !lib_dir.exists() {
        return Err(io::Error::other(format!(
            "NCCL build output was missing include/ or lib/ under {}",
            build_dir.display()
        )));
    }

    let install_dir_path = Path::new(install_dir);
    if install_dir_path.exists() {
        if install_dir_path.is_dir() {
            fs::remove_dir_all(install_dir_path)?;
        } else {
            fs::remove_file(install_dir_path)?;
        }
    }

    fs::create_dir_all(install_dir_path)?;

    let include = include_dir.to_string_lossy().into_owned();
    let lib = lib_dir.to_string_lossy().into_owned();
    run_cmd(
        "cp",
        ["-a", include.as_str(), lib.as_str(), install_dir],
        CommandOptions::default(),
    )?;

    fs::write(
        install_dir_path.join("VERSION"),
        format!("NCCL {NCCL_VERSION}\n"),
    )?;

    Ok(())
}

fn configure_nccl_environment(install_dir: &str) -> io::Result<()> {
    let mut profile = File::create(NCCL_PROFILE_FILENAME)?;
    writeln!(
        profile,
        "# Configuring NCCL. File created by Spyral CUDA installation manager."
    )?;
    for export in nccl_env_exports(install_dir) {
        writeln!(profile, "{export}")?;
    }

    Ok(())
}

fn nccl_env_exports(install_dir: &str) -> [String; 4] {
    [
        format!("export NCCL_HOME={install_dir}"),
        format!("export CPATH={install_dir}/include${{CPATH:+:${{CPATH}}}}"),
        format!("export LIBRARY_PATH={install_dir}/lib${{LIBRARY_PATH:+:${{LIBRARY_PATH}}}}"),
        format!(
            "export LD_LIBRARY_PATH={install_dir}/lib${{LD_LIBRARY_PATH:+:${{LD_LIBRARY_PATH}}}}"
        ),
    ]
}

fn verify_nccl_installation(install_dir: &str) -> io::Result<()> {
    let header_path = Path::new(install_dir).join("include/nccl.h");
    let library_path = Path::new(install_dir).join("lib/libnccl.so");

    if !header_path.exists() || !library_path.exists() {
        return Err(io::Error::other(format!(
            "NCCL installation verification failed. Expected {} and {} to exist.",
            header_path.display(),
            library_path.display()
        )));
    }

    Ok(())
}

pub(crate) fn detect_cuda_home() -> io::Result<String> {
    let default_cuda = Path::new("/usr/local/cuda");
    if default_cuda.exists() {
        return Ok(default_cuda.display().to_string());
    }

    let output = run_cmd(
        "which",
        ["nvcc"],
        CommandOptions {
            check: false,
            silent: true,
            ..Default::default()
        },
    )?;
    if output.status.success() {
        let nvcc_path = PathBuf::from(output.stdout.trim());
        if let Some(cuda_home) = nvcc_path.parent().and_then(Path::parent) {
            return Ok(cuda_home.display().to_string());
        }
    }

    if let Ok(content) = fs::read_to_string(PROFILE_FILENAME) {
        for line in content.lines() {
            if let Some(path_export) = line.strip_prefix("export PATH=") {
                let path_prefix = path_export.split("${").next().unwrap_or("");
                if let Some(cuda_bin) = path_prefix.strip_suffix("/bin") {
                    let cuda_home = Path::new(cuda_bin);
                    if cuda_home.exists() {
                        return Ok(cuda_home.display().to_string());
                    }
                }
            }
        }
    }

    let mut cuda_dirs: Vec<String> = fs::read_dir("/usr/local")?
        .filter_map(|entry| {
            let entry = entry.ok()?;
            if !entry.path().is_dir() {
                return None;
            }

            let name = entry.file_name().into_string().ok()?;
            if name.starts_with("cuda-") {
                Some(format!("/usr/local/{name}"))
            } else {
                None
            }
        })
        .collect();
    cuda_dirs.sort();

    cuda_dirs
        .pop()
        .ok_or_else(|| io::Error::other("Could not locate a CUDA installation for building NCCL"))
}

fn cuda_postinstallation_actions(cuda_config: &CudaConfig) -> io::Result<()> {
    // Set environment variables for the current process
    env::set_var(
        "PATH",
        format!(
            "{}:{}",
            cuda_config.bin_folder,
            env::var("PATH").unwrap_or_default()
        ),
    );

    if let Ok(ld_library_path) = env::var("LD_LIBRARY_PATH") {
        env::set_var(
            "LD_LIBRARY_PATH",
            format!("{}:{}", cuda_config.lib_folder, ld_library_path),
        );
    } else {
        env::set_var("LD_LIBRARY_PATH", &cuda_config.lib_folder);
    }

    // Create profile file for persistent environment variables
    let mut profile = File::create(PROFILE_FILENAME)?;
    writeln!(
        profile,
        "# Configuring CUDA toolkit. File created by Spyral CUDA installation manager."
    )?;
    writeln!(
        profile,
        "export PATH={}${{PATH:+:${{PATH}}}}",
        cuda_config.bin_folder
    )?;
    writeln!(
        profile,
        "export LD_LIBRARY_PATH={}${{LD_LIBRARY_PATH:+:${{LD_LIBRARY_PATH}}}}",
        cuda_config.lib_folder
    )?;

    configure_persistanced_service()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn latest_cuda_is_the_default() {
        assert_eq!(CudaVersion::default(), CudaVersion::V13_3_1);
    }

    #[test]
    fn cuda_13_0_uses_the_pinned_gcp_kernel() {
        let kernel =
            KernelConfig::for_platform("ubuntu", "24.04", CloudProvider::Gcp, CudaVersion::V13_0_1)
                .unwrap();

        assert_eq!(kernel.version, "6.17.0-1022-gcp");
        assert_eq!(kernel.image.version, "6.17.0-1022.25");
        assert_eq!(kernel.modules.name, "linux-modules-6.17.0-1022-gcp");
    }

    #[test]
    fn cuda_13_0_uses_the_pinned_aws_kernel() {
        let kernel =
            KernelConfig::for_platform("ubuntu", "24.04", CloudProvider::Aws, CudaVersion::V13_0_1)
                .unwrap();

        assert_eq!(kernel.version, "6.17.0-1020-aws");
        assert_eq!(kernel.image.version, "6.17.0-1020.20~24.04.1+1");
        assert_eq!(kernel.headers.version, "6.17.0-1020.20~24.04.1");
        assert_eq!(kernel.modules.name, "linux-modules-6.17.0-1020-aws");
    }

    #[test]
    fn cuda_13_3_uses_the_kernel_7_gcp_profile() {
        let kernel =
            KernelConfig::for_platform("ubuntu", "24.04", CloudProvider::Gcp, CudaVersion::V13_3_1)
                .unwrap();

        assert_eq!(kernel.version, "7.0.0-1011-gcp");
        assert_eq!(kernel.image.version, "7.0.0-1011.11~24.04.1");
        assert_eq!(kernel.modules.name, "linux-modules-7.0.0-1011-gcp");
    }

    #[test]
    fn cuda_13_3_uses_the_kernel_7_aws_profile() {
        let kernel =
            KernelConfig::for_platform("ubuntu", "24.04", CloudProvider::Aws, CudaVersion::V13_3_1)
                .unwrap();

        assert_eq!(kernel.version, "7.0.0-1012-aws");
        assert_eq!(kernel.image.version, "7.0.0-1012.12~24.04.1");
        assert_eq!(kernel.modules.name, "linux-modules-7.0.0-1012-aws");
    }

    #[test]
    fn newer_cuda_releases_match_their_installers() {
        for (version, release, driver, checksum) in [
            (
                CudaVersion::V13_1_1,
                "13.1.1",
                "590.48.01",
                "8aa93a77cffa8d055db0ceb9d0e2d692",
            ),
            (
                CudaVersion::V13_2_1,
                "13.2.1",
                "595.58.03",
                "e5b4bdf19cc27d63a8254cb486764626",
            ),
            (
                CudaVersion::V13_3_1,
                "13.3.1",
                "610.43.02",
                "7c8d3eca60ee10d2c290bdc045f88f09",
            ),
        ] {
            let cuda = CudaConfig::new(version);

            assert!(cuda.toolkit_url.contains(release));
            assert!(cuda.toolkit_url.contains(driver));
            assert_eq!(cuda.driver_version, driver);
            assert_eq!(cuda.toolkit_checksum, checksum);
        }
    }

    #[test]
    fn cuda_12_8_driver_version_matches_its_installer() {
        let cuda = CudaConfig::new(CudaVersion::V12_8);

        assert_eq!(cuda.driver_version, "570.86.10");
        assert!(cuda.toolkit_url.contains(&cuda.driver_version));
    }

    #[test]
    fn unvalidated_distribution_is_rejected() {
        let result =
            KernelConfig::for_platform("ubuntu", "22.04", CloudProvider::Gcp, CudaVersion::V13_0_1);

        assert!(result.is_err());
    }
}
