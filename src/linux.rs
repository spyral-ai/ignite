use std::{env, fs, io, path::Path};

use crate::utils::{run_cmd, CommandOptions};

const KERNEL_APT_CONFIG: &str = "/etc/apt/apt.conf.d/60ignite-kernel";
const KERNEL_GRUB_CONFIG: &str = "/etc/default/grub.d/99-ignite-kernel.cfg";
const RESUME_BINARY: &str = "/usr/local/sbin/ignite";
const RESUME_SERVICE: &str = "ignite-cuda-resume.service";
const RESUME_SERVICE_PATH: &str = "/etc/systemd/system/ignite-cuda-resume.service";

pub(crate) struct KernelPackage {
    pub(crate) name: String,
    pub(crate) version: &'static str,
}

pub(crate) struct KernelConfig {
    pub(crate) version: &'static str,
    pub(crate) flavor: &'static str,
    pub(crate) image: KernelPackage,
    pub(crate) headers: KernelPackage,
    pub(crate) modules: KernelPackage,
    pub(crate) rolling_packages: [String; 3],
}

impl KernelConfig {
    pub(crate) fn new(
        version: &'static str,
        flavor: &'static str,
        image_version: &'static str,
        headers_version: &'static str,
        modules_version: &'static str,
    ) -> Self {
        let package = |kind, package_version| KernelPackage {
            name: format!("linux-{kind}-{version}"),
            version: package_version,
        };

        Self {
            version,
            flavor,
            image: package("image", image_version),
            headers: package("headers", headers_version),
            modules: package("modules", modules_version),
            rolling_packages: [
                format!("linux-{flavor}"),
                format!("linux-image-{flavor}"),
                format!("linux-headers-{flavor}"),
            ],
        }
    }

    fn required_packages(&self) -> [&KernelPackage; 3] {
        [&self.image, &self.headers, &self.modules]
    }
}
pub(crate) fn kernel_version() -> io::Result<String> {
    let output = run_cmd("uname", ["-r"], CommandOptions::default())?;
    Ok(output.stdout.trim().to_string())
}

pub(crate) fn install_pinned_kernel(kernel: &KernelConfig) -> io::Result<bool> {
    run_cmd("apt-get", ["update"], CommandOptions::default())?;

    let mut packages = vec![
        "build-essential".to_string(),
        "dkms".to_string(),
        "software-properties-common".to_string(),
        "pciutils".to_string(),
    ];
    for package in kernel.required_packages() {
        packages.push(format!("{}={}", package.name, package.version));
    }

    let mut all_packages_installed = true;
    for package in kernel.required_packages() {
        all_packages_installed &= package_is_installed(&package.name, Some(package.version))?;
    }
    for package in [
        "build-essential",
        "dkms",
        "software-properties-common",
        "pciutils",
    ] {
        all_packages_installed &= package_is_installed(package, None)?;
    }

    if !all_packages_installed {
        let mut args = vec!["install".to_string(), "-y".to_string()];
        args.extend(packages);
        run_cmd("apt-get", args, CommandOptions::default())?;
    }

    configure_kernel_pin(kernel)?;

    let current_kernel = kernel_version()?;
    Ok(current_kernel != kernel.version)
}

pub(crate) fn remove_kernel_pin(kernel: &KernelConfig) -> io::Result<()> {
    for path in [KERNEL_APT_CONFIG, KERNEL_GRUB_CONFIG] {
        match fs::remove_file(path) {
            Ok(()) => {}
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => return Err(error),
        }
    }

    let mut held_packages = Vec::new();
    for package in kernel.rolling_packages.iter().map(String::as_str).chain(
        kernel
            .required_packages()
            .into_iter()
            .map(|package| package.name.as_str()),
    ) {
        if package_is_present(package)? {
            held_packages.push(package);
        }
    }
    if !held_packages.is_empty() {
        let mut args = vec!["unhold"];
        args.extend(held_packages);
        run_cmd("apt-mark", args, CommandOptions::default())?;
    }
    run_cmd(
        "update-grub",
        std::iter::empty::<&str>(),
        CommandOptions::default(),
    )?;

    Ok(())
}

/// Copies the ignite binary to persistent storage and enables a one-shot service that
/// invokes it with `arguments` after the system reboots.
///
/// # Errors
///
/// Returns an error if the executable cannot be located or copied, the service
/// file cannot be written, or systemd cannot enable the service.
pub(crate) fn configure_resume_service(arguments: &[&str]) -> io::Result<()> {
    let current_binary = env::current_exe()?;
    let resume_binary = Path::new(RESUME_BINARY);
    if current_binary != resume_binary {
        fs::copy(&current_binary, resume_binary)?;
    }

    fs::write(
        RESUME_SERVICE_PATH,
        format!(
            "[Unit]\n\
             Description=Continue an Ignite installation after a kernel reboot\n\
             Wants=network-online.target\n\
             After=network-online.target\n\
             \n\
             [Service]\n\
             Type=oneshot\n\
             ExecStart={RESUME_BINARY} {}\n\
             \n\
             [Install]\n\
             WantedBy=multi-user.target\n",
            arguments.join(" "),
        ),
    )?;
    run_cmd(
        "systemctl",
        ["enable", RESUME_SERVICE],
        CommandOptions::default(),
    )?;

    Ok(())
}

pub(crate) fn clear_resume_service() -> io::Result<()> {
    if !Path::new(RESUME_SERVICE_PATH).exists() {
        return Ok(());
    }

    run_cmd(
        "systemctl",
        ["disable", RESUME_SERVICE],
        CommandOptions {
            check: false,
            silent: true,
            ..Default::default()
        },
    )?;
    fs::remove_file(RESUME_SERVICE_PATH)?;
    run_cmd("systemctl", ["daemon-reload"], CommandOptions::default())?;

    Ok(())
}

pub(crate) fn distro_id() -> io::Result<String> {
    os_release_value("ID")
}

pub(crate) fn distro_version() -> io::Result<String> {
    os_release_value("VERSION_ID")
}

pub(crate) fn reboot() -> ! {
    println!("The system needs to be rebooted to complete the installation process.");

    match run_cmd("reboot", ["now"], CommandOptions::default()) {
        Ok(_) => std::process::exit(0),
        Err(error) => {
            eprintln!("Failed to reboot: {error}");
            std::process::exit(1);
        }
    }
}

fn package_is_installed(package: &str, version: Option<&str>) -> io::Result<bool> {
    let output = run_cmd(
        "dpkg-query",
        ["-W", "-f=${db:Status-Abbrev} ${Version}", package],
        CommandOptions {
            check: false,
            silent: true,
            ..Default::default()
        },
    )?;
    if !output.status.success() {
        return Ok(false);
    }

    let mut fields = output.stdout.split_whitespace();
    let status = fields.next();
    let installed_version = fields.next();
    Ok(status == Some("ii") && version.is_none_or(|expected| installed_version == Some(expected)))
}

fn configure_kernel_pin(kernel: &KernelConfig) -> io::Result<()> {
    fs::write(
        KERNEL_APT_CONFIG,
        format!(
            "// Generated by Ignite. CUDA hosts advance kernels through an Ignite release.\n\
             Unattended-Upgrade::Package-Blacklist {{\n\
             \"^linux-{0}$\";\n\
             \"^linux-image-{0}$\";\n\
             \"^linux-headers-{0}$\";\n\
             \"^linux-image-[0-9].*-{0}$\";\n\
             \"^linux-headers-[0-9].*-{0}$\";\n\
             \"^linux-modules(-extra)?-[0-9].*-{0}$\";\n\
             }};\n",
            kernel.flavor
        ),
    )?;

    let grub_config_dir = Path::new(KERNEL_GRUB_CONFIG)
        .parent()
        .ok_or_else(|| io::Error::other("The Ignite GRUB configuration path has no parent"))?;
    fs::create_dir_all(grub_config_dir)?;
    fs::write(
        KERNEL_GRUB_CONFIG,
        format!(
            "# Generated by Ignite.\nGRUB_DEFAULT=\"Advanced options for Ubuntu>Ubuntu, with Linux {}\"\n",
            kernel.version
        ),
    )?;
    run_cmd(
        "update-grub",
        std::iter::empty::<&str>(),
        CommandOptions::default(),
    )?;

    let required_packages = kernel.required_packages();
    let mut manual_args = vec!["manual"];
    manual_args.extend(
        required_packages
            .iter()
            .map(|package| package.name.as_str()),
    );
    run_cmd("apt-mark", manual_args, CommandOptions::default())?;

    let mut held_packages = Vec::new();
    for package in kernel.rolling_packages.iter().map(String::as_str).chain(
        kernel
            .required_packages()
            .into_iter()
            .map(|package| package.name.as_str()),
    ) {
        if package_is_present(package)? {
            held_packages.push(package);
        }
    }
    if !held_packages.is_empty() {
        let mut args = vec!["hold"];
        args.extend(held_packages);
        run_cmd("apt-mark", args, CommandOptions::default())?;
    }

    Ok(())
}

fn package_is_present(package: &str) -> io::Result<bool> {
    let output = run_cmd(
        "dpkg-query",
        ["-W", "-f=${db:Status-Abbrev}", package],
        CommandOptions {
            check: false,
            silent: true,
            ..Default::default()
        },
    )?;

    Ok(output.status.success() && package_status_is_installed(&output.stdout))
}

fn package_status_is_installed(status: &str) -> bool {
    status.as_bytes().get(1) == Some(&b'i')
}

fn os_release_value(key: &str) -> io::Result<String> {
    let content = fs::read_to_string("/etc/os-release")?;
    let prefix = format!("{key}=");
    content
        .lines()
        .find_map(|line| line.strip_prefix(&prefix))
        .map(|value| value.trim_matches('"').to_string())
        .ok_or_else(|| io::Error::other(format!("Could not determine {key} from /etc/os-release")))
}
