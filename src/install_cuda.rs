use std::{
    env,
    fs::{self, File},
    io::{self, Write},
    path::{Path, PathBuf},
};

use clap::ValueEnum;
use tempfile::TempDir;

use crate::{utils::*, CloudProvider};

const PROFILE_FILENAME: &str = "/etc/profile.d/spyral_cuda_install.sh";
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
                toolkit_url: String::from("https://developer.download.nvidia.com/compute/cuda/12.5.0/local_installers/cuda_12.5.0_555.42.02_linux.run"),
                toolkit_checksum: String::from("0bf587ce20c8e74b90701be56ae2c907"),
                bin_folder: String::from("/usr/local/cuda-12.5/bin"),
                lib_folder: String::from("/usr/local/cuda-12.5/lib64"),
                driver_version: String::from("555.42.02"),
            },
            CudaVersion::V12_6 => Self {
                version,
                toolkit_url: String::from("https://developer.download.nvidia.com/compute/cuda/12.6.0/local_installers/cuda_12.6.0_560.28.03_linux.run"),
                toolkit_checksum: String::from("8685a58497b0c7e5d964e6da7968bb1e"),
                bin_folder: String::from("/usr/local/cuda-12.6/bin"),
                lib_folder: String::from("/usr/local/cuda-12.6/lib64"),
                driver_version: String::from("535.161.07"),
            },

            CudaVersion::V12_8 => Self {
                version,
                toolkit_url: String::from("https://developer.download.nvidia.com/compute/cuda/12.8.0/local_installers/cuda_12.8.0_570.86.10_linux.run"),
                toolkit_checksum: String::from("c71027cf1a4ce84f80b9cbf81116e767"),
                bin_folder: String::from("/usr/local/cuda-12.8/bin"),
                lib_folder: String::from("/usr/local/cuda-12.8/lib64"),
                driver_version: String::from("550.54.14"),
            },
            CudaVersion::V13_0_1 => Self {
                version,
                toolkit_url: String::from("https://developer.download.nvidia.com/compute/cuda/13.0.1/local_installers/cuda_13.0.1_580.82.07_linux.run"),
                toolkit_checksum: String::from("8c56e3cb1ab74370aafed5a4600bc5bc"),
                bin_folder: String::from("/usr/local/cuda-13.0.1/bin"),
                lib_folder: String::from("/usr/local/cuda-13.0.1/lib64"),
                driver_version: String::from("580.82.07")
            }
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
    run(&format!(
        "sh {} --silent --driver",
        installer_path.display()
    ))?;

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
    run(&format!(
        "sh {} --extract={}",
        installer_path.display(),
        temp_dir.path().display()
    ))?;

    let installer_path = temp_dir.path().join(format!(
        "NVIDIA-Linux-x86_64-{}.run",
        cuda_config.driver_version
    ));

    println!("Starting uninstallation...");
    run(&format!("sh {} -s --uninstall", installer_path.display()))?;

    println!("Uninstallation completed!");
    unlock_kernel_updates_debian()?;

    Ok(())
}

pub(crate) fn verify_driver(verbose: bool) -> io::Result<bool> {
    let (status, _, _) = run_with_options("which nvidia-smi", false, None, true, 0)?;

    if !status.success() {
        if verbose {
            println!("Couldn't find nvidia-smi, the driver is not installed.");
        }
        return Ok(false);
    }

    let (status, stdout, stderr) = run_with_options("nvidia-smi -L", false, None, true, 0)?;
    let success = status.success() && stdout.contains("UUID");

    if verbose {
        println!("nvidia-smi -L output: {} {}", stdout, stderr);
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
    run(&format!(
        "sh {} --silent --toolkit",
        installer_path.display()
    ))
    .unwrap();
    println!("CUDA toolkit installation completed!");

    println!("Executing post-installation actions...");
    cuda_postinstallation_actions(&cuda_config).unwrap();
    println!("CUDA post-installation actions completed!");

    Err(RebootRequired)
}

fn install_dependencies_debian(cloud_provider: CloudProvider) -> Result<(), RebootRequired> {
    let kernel_suffix = cloud_provider.kernel_suffix();
    let kernel_image_package = "linux-image-{version}";
    let kernel_version_format = format!("{{major}}.{{minor}}.{{patch}}-{{micro}}{}", kernel_suffix);
    let kernel_headers_package = "linux-headers-{version}";

    run("apt-get update").unwrap();

    let kernel_version = get_kernel_version().unwrap();
    let mut version_parts = kernel_version.split('.');
    let major = version_parts.next().unwrap();
    let minor = version_parts.next().unwrap();
    println!("Major: {major}, minor: {minor}");

    // Get all available linux-image packages
    let (_, packages, _) = run("apt-cache search linux-image").unwrap();

    // Find the newest version matching our major.minor
    let prefix = format!("linux-image-{}.{}", major, minor);
    println!("Searching for prefix: {prefix}");

    let mut max_patch = 0;
    let mut max_micro = 0;

    for line in packages.lines() {
        let package_name = line.split_whitespace().next().unwrap_or("");
        println!("Package: {package_name}");
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

    // Check if the wanted kernel is already installed
    let current_kernel = get_kernel_version().unwrap();
    let is_kernel_installed =
        current_kernel.contains(&format!("{}.{}.{}-{}", major, minor, max_patch, max_micro));

    // Check if the headers are already installed
    let (status, _, _) = run_with_options(
        &format!("dpkg -l | grep {}", wanted_kernel_headers),
        false,
        None,
        true,
        0,
    )
    .unwrap();
    let are_headers_installed = status.success();

    // If both kernel and headers are already installed, no need to reboot
    if is_kernel_installed && are_headers_installed {
        println!("Required kernel and headers are already installed.");
        return Ok(());
    }

    // Install the packages
    run(&format!(
        "apt-get install -y make gcc {} {} software-properties-common pciutils gcc make dkms",
        wanted_kernel_package, wanted_kernel_headers
    ))
    .unwrap();

    // Reboot is needed only if we installed a new kernel
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

    run_with_options("tar -xf installer.tar.bz2", true, None, true, 0)?;
    println!("Executing nvidia-persistenced installer...");
    run("sh nvidia-persistenced-init/install.sh")?;

    env::set_current_dir(current_dir)?;
    Ok(())
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
