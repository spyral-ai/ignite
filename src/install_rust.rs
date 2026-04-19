use std::{
    env, fs,
    io::{self, Write},
    path::Path,
};

use crate::utils::{run_cmd, CommandOptions};

pub fn install_rust(home_dir: String) -> io::Result<()> {
    if Path::new(&format!("{home_dir}/.cargo/bin")).exists() {
        println!("Rust is already installed. Skipping installation.");
        return Ok(());
    }

    println!("Installing Rust...");

    // Install dependencies
    run_cmd("apt-get", ["update"], CommandOptions::default())?;
    run_cmd(
        "apt-get",
        ["install", "-y", "curl", "build-essential"],
        CommandOptions::default(),
    )?;

    // Download and run rustup installer
    println!("Downloading and running rustup installer...");
    run_cmd(
        "curl",
        [
            "-tlsv1.2",
            "-sSf",
            "https://sh.rustup.rs",
            "-o",
            "/tmp/sh.rustup.rs",
        ],
        CommandOptions::default(),
    )?;
    run_cmd("sh", ["/tmp/sh.rustup.rs", "-y"], CommandOptions::default())?;

    // Get the home directory and sudo user
    let sudo_user = env::var("SUDO_USER").unwrap_or_else(|_| String::from(""));

    // Add Cargo to PATH permanently by updating shell configuration files
    println!("Adding Cargo to PATH...");
    let config_file = format!("{}/.bashrc", home_dir);
    if Path::new(&config_file).exists() {
        // Check if the PATH entry already exists
        let content = fs::read_to_string(&config_file)?;
        if !content.contains(".cargo/bin") {
            println!("Updating {config_file}");
            let mut file = fs::OpenOptions::new().append(true).open(&config_file)?;

            writeln!(file, "\n# Add Rust's cargo to PATH")?;
            writeln!(file, "export PATH=\"$HOME/.cargo/bin:$PATH\"")?;
        }
    }

    // Fix permissions if running as root for a regular user
    if !sudo_user.is_empty() {
        println!("Setting correct ownership for Rust installation...");
        let owner = format!("{sudo_user}:{sudo_user}");
        let cargo_dir = format!("{home_dir}/.cargo");
        run_cmd(
            "chown",
            ["-R", owner.as_str(), cargo_dir.as_str()],
            CommandOptions::default(),
        )?;

        // Also fix permissions for the shell config files
        if Path::new(&config_file).exists() {
            run_cmd(
                "chown",
                [owner.as_str(), config_file.as_str()],
                CommandOptions::default(),
            )?;
        }
    }

    let rustup_path = format!("{home_dir}/.cargo/bin/rustup");
    println!("Installing rust components, rustup path: {rustup_path} ...");
    run_cmd(
        rustup_path.as_str(),
        ["toolchain", "install", "nightly"],
        CommandOptions::default(),
    )?;
    run_cmd(
        rustup_path.as_str(),
        ["default", "nightly"],
        CommandOptions::default(),
    )?;
    run_cmd(
        rustup_path.as_str(),
        ["component", "add", "rust-analyzer"],
        CommandOptions::default(),
    )?;

    println!("Rust installation completed successfully!");

    Ok(())
}
