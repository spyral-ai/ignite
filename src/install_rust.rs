use crate::utils::run;
use std::{env, io};

pub fn install_rust() -> io::Result<()> {
    println!("Installing Rust...");

    // Install dependencies
    run("apt-get update", true, None, false, 0)?;
    run(
        "apt-get install -y curl build-essential",
        true,
        None,
        false,
        0,
    )?;

    // Download and run rustup installer
    println!("Downloading and running rustup installer...");

    // Get the current user to run rustup as
    let current_user = if let Ok(sudo_user) = env::var("SUDO_USER") {
        sudo_user
    } else {
        String::from("root")
    };

    // Run rustup installer with default settings
    run(
        &format!(
            "curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | su - {} -c 'sh -s -- -y'",
            current_user
        ),
        true,
        None,
        false,
        0,
    )?;

    // Add Rust to PATH for the current session
    let home_dir = if current_user == "root" {
        String::from("/root")
    } else {
        format!("/home/{}", current_user)
    };

    // Source the cargo env file to update PATH
    run(
        &format!("su - {} -c 'source {0}/.cargo/env'", current_user, home_dir),
        false, // Don't check exit status as 'source' is a shell builtin
        None,
        false,
        0,
    )?;

    println!("Rust installation completed successfully!");
    println!("Note: You may need to restart your shell or run 'source ~/.cargo/env' to use Rust.");

    Ok(())
}
