use std::{
    env,
    fs::{self, File},
    io::{self, Write},
    path::{Path, PathBuf},
};

use clap::{Args, ValueEnum};
use tempfile::TempDir;

use crate::{utils::*, CloudProvider};

const PROFILE_FILENAME: &str = "/etc/profile.d/spyral_cuda_install.sh";
const NCCL_PROFILE_FILENAME: &str = "/etc/profile.d/spyral_nccl.sh";
const DEFAULT_NCCL_INSTALL_DIR: &str = "/opt/nccl";
const NCCL_VERSION: &str = "2.30.3-1";
const NCCL_SOURCE_URL: &str = "https://github.com/NVIDIA/nccl/archive/refs/tags/v2.30.3-1.tar.gz";
const NVIDIA_PERSISTANCED_INSTALLER: &str =
    "/usr/share/doc/NVIDIA_GLX-1.0/samples/nvidia-persistenced-init.tar.bz2";

struct RebootRequired;

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum CudaVersion {
    V12_5,
    V12_6,
    V12_8,
    V13_0_1,
}

impl std::fmt::Display for CudaVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CudaVersion::V12_5 => write!(f, "12.5"),
            CudaVersion::V12_6 => write!(f, "12.6"),
            CudaVersion::V12_8 => write!(f, "12.8"),
            CudaVersion::V13_0_1 => write!(f, "13.0.1"),
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
                driver_version: String::from("550.54.14"),
            },
            CudaVersion::V13_0_1 => Self {
                version,
                toolkit_url: String::from(
                    "https://developer.download.nvidia.com/compute/cuda/13.0.1/local_installers/cuda_13.0.1_580.82.07_linux.run",
                ),
                toolkit_checksum: String::from("8c56e3cb1ab74370aafed5a4600bc5bc"),
                bin_folder: String::from("/usr/local/cuda-13.0.1/bin"),
                lib_folder: String::from("/usr/local/cuda-13.0.1/lib64"),
                driver_version: String::from("580.82.07"),
            },
        }
    }
}

pub(crate) fn install_driver(
    cloud_provider: CloudProvider,
    cuda_version: CudaVersion,
) -> io::Result<()> {
    let cuda_config = CudaConfig::new(cuda_version);

    match install_dependencies_debian(cloud_provider) {
        Ok(_) => {
            println!("Dependencies installed successfully without requiring a reboot.");
        }
        Err(RebootRequired) => {
            println!("System will reboot to apply kernel changes.");
            reboot();
        }
    }

    println!("Installing GPU drivers for CUDA {}...", cuda_version);

    let installer_path = download_cuda_toolkit_installer(&cuda_config)?;
    let installer = installer_path.to_string_lossy().into_owned();
    run_cmd(
        "sh",
        [installer.as_str(), "--silent", "--driver"],
        CommandOptions::default(),
    )?;

    if verify_driver(true)? {
        lock_kernel_updates_debian()?;
        println!("GPU driver installed successfully!");
    } else {
        println!("Something went wrong with driver installation, installation failed");
    }

    Ok(())
}

pub(crate) fn uninstall_driver(cuda_version: CudaVersion) -> io::Result<()> {
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
    unlock_kernel_updates_debian()?;

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

pub(crate) fn install_cuda(
    cloud_provider: CloudProvider,
    cuda_version: CudaVersion,
) -> io::Result<()> {
    match install_cuda_inner(cloud_provider, cuda_version) {
        Ok(_) => Ok(()),
        Err(RebootRequired) => {
            reboot();
        }
    }
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
    let build_result = (|| {
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
    })();
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

fn install_cuda_inner(
    cloud_provider: CloudProvider,
    cuda_version: CudaVersion,
) -> Result<(), RebootRequired> {
    let cuda_config = CudaConfig::new(cuda_version);

    if !verify_driver(false).unwrap_or(false) {
        println!(
            "CUDA installation requires GPU driver to be installed first. \
            Attempting to install GPU driver now."
        );
        install_driver(cloud_provider, cuda_version).unwrap();
    }

    if Path::new(&format!("{}/nvcc", cuda_config.bin_folder)).exists() {
        println!(
            "Nvcc already installed at : {}/nvcc, not installing CUDA",
            cuda_config.bin_folder
        );
        return Ok(());
    }

    let installer_path = download_cuda_toolkit_installer(&cuda_config).unwrap();

    println!("Installing CUDA {} toolkit...", cuda_version);
    let installer = installer_path.to_string_lossy().into_owned();
    run_cmd(
        "sh",
        [installer.as_str(), "--silent", "--toolkit"],
        CommandOptions::default(),
    )
    .unwrap();
    println!("CUDA toolkit installation completed!");

    println!("Executing post-installation actions...");
    cuda_postinstallation_actions(&cuda_config).unwrap();
    println!("CUDA post-installation actions completed!");

    Ok(())
}

fn install_dependencies_debian(cloud_provider: CloudProvider) -> Result<(), RebootRequired> {
    let distro_id = get_distro_id().unwrap();
    let kernel_suffix = cloud_provider.kernel_suffix(&distro_id);
    let kernel_image_package = "linux-image-{version}";
    let kernel_version_format = format!("{{major}}.{{minor}}.{{patch}}-{{micro}}{}", kernel_suffix);
    let kernel_headers_package = "linux-headers-{version}";
    let kernel_modules_extra_package = "linux-modules-extra-{version}";

    run_cmd("apt-get", ["update"], CommandOptions::default()).unwrap();

    let kernel_version = get_kernel_version().unwrap();
    let mut version_parts = kernel_version.split('.');
    let major = version_parts.next().unwrap();
    let minor = version_parts.next().unwrap();
    println!("Major: {major}, minor: {minor}");

    // Get all available linux-image packages
    let packages = run_cmd(
        "apt-cache",
        ["search", "linux-image"],
        CommandOptions::default(),
    )
    .unwrap()
    .stdout;

    // Find the newest version matching our major.minor
    let prefix = format!("linux-image-{}.{}", major, minor);
    println!("Searching for prefix: {prefix}");

    let mut max_patch = 0;
    let mut max_micro = 0;

    for line in packages.lines() {
        let package_name = line.split_whitespace().next().unwrap_or("");
        if let Some((patch, micro)) = parse_kernel_package(package_name, &prefix, kernel_suffix) {
            if patch > max_patch || (patch == max_patch && micro > max_micro) {
                max_patch = patch;
                max_micro = micro;
            }
        }
    }

    let wanted_kernel_version = kernel_version_format
        .replace("{major}", major)
        .replace("{minor}", minor)
        .replace("{patch}", &max_patch.to_string())
        .replace("{micro}", &max_micro.to_string());
    println!("Wanted kernel version: {wanted_kernel_version}");

    let wanted_kernel_package = kernel_image_package.replace("{version}", &wanted_kernel_version);
    let wanted_kernel_headers = kernel_headers_package.replace("{version}", &wanted_kernel_version);
    let wanted_kernel_modules_extra =
        kernel_modules_extra_package.replace("{version}", &wanted_kernel_version);

    // Check if the wanted kernel is already installed
    let current_kernel = get_kernel_version().unwrap();
    let is_kernel_installed =
        current_kernel.contains(&format!("{}.{}.{}-{}", major, minor, max_patch, max_micro));

    // Check if the headers are already installed
    let headers_status = run_cmd(
        "dpkg",
        ["-s", wanted_kernel_headers.as_str()],
        CommandOptions {
            check: false,
            silent: true,
            ..Default::default()
        },
    )
    .unwrap()
    .status;
    let are_headers_installed = headers_status.success();

    let modules_status = run_cmd(
        "dpkg",
        ["-s", wanted_kernel_modules_extra.as_str()],
        CommandOptions {
            check: false,
            silent: true,
            ..Default::default()
        },
    )
    .unwrap()
    .status;
    let are_modules_extra_installed = modules_status.success();

    // If both kernel and headers are already installed, no need to reboot
    if is_kernel_installed && are_headers_installed && are_modules_extra_installed {
        println!("Required kernel, headers, and exra modules are already installed.");
        return Ok(());
    }

    // Install the packages
    run_cmd(
        "apt-get",
        [
            "install",
            "-y",
            wanted_kernel_package.as_str(),
            wanted_kernel_headers.as_str(),
            wanted_kernel_modules_extra.as_str(),
            "build-essential",
            "dkms",
            "software-properties-common",
            "pciutils",
        ],
        CommandOptions::default(),
    )
    .unwrap();

    if !is_kernel_installed {
        println!("New kernel installed. System needs to reboot.");
        Err(RebootRequired)
    } else {
        println!("Kernel already matches required version. No reboot needed.");
        Ok(())
    }
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

fn detect_cuda_home() -> io::Result<String> {
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
