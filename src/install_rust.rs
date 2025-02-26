use std::{
    env, fs,
    io::{self, Write},
    path::Path,
};

use crate::utils::run;

pub fn install_rust(home_dir: String) -> io::Result<()> {
    if Path::new(&format!("{home_dir}/.cargo/bin")).exists() {
        println!("Rust is already installed. Skipping installation.");
        return Ok(());
    }

    println!("Installing Rust...");

    // Install dependencies
    run("apt-get update")?;
    run("apt-get install -y curl build-essential")?;

    // Download and run rustup installer
    println!("Downloading and running rustup installer...");
    run("curl -tlsv1.2 -sSf https://sh.rustup.rs -o /tmp/sh.rustup.rs")?;
    run("sh /tmp/sh.rustup.rs -y")?;

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

            run(&format!("source {config_file}"))?;
        }
    }

    // Fix permissions if running as root for a regular user
    if !sudo_user.is_empty() {
        println!("Setting correct ownership for Rust installation...");
        run(&format!(
            "chown -R {sudo_user}:{sudo_user} {home_dir}/.cargo"
        ))?;

        // Also fix permissions for the shell config files
        if Path::new(&config_file).exists() {
            run(&format!("chown {sudo_user}:{sudo_user} {config_file}"))?;
        }
    }

    let rustup_path = format!("{home_dir}/.cargo/bin/rustup");
    println!("Installing rust components, rustup path: {rustup_path} ...");
    run(&format!("{rustup_path} toolchain install nightly"))?;
    run(&format!("{rustup_path} default nightly"))?;
    run(&format!("{rustup_path} component add rust-analyzer"))?;

    println!("Rust installation completed successfully!");

    Ok(())
}
