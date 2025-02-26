use crate::utils::run;
use std::{env, fs, io, path::Path};

pub fn install_nvim(home_dir: String) -> io::Result<()> {
    println!("Installing Neovim and dependencies...");

    // Install dependencies
    run("apt-get install -y git-all", true, None, false, 0)?;
    run("apt-get install -y lua5.4", true, None, false, 0)?;
    run("apt-get install -y unzip", true, None, false, 0)?;
    run("apt-get install -y npm", true, None, false, 0)?;

    run(
        "curl -fsSL https://deno.land/install.sh -o /tmp/deno_install.sh",
        true,
        None,
        false,
        0,
    )?;
    run("sh /tmp/deno_install.sh", true, None, false, 0)?;

    // Create config directory if it doesn't exist
    println!("Installing neovim with home dir: {home_dir}");
    let config_dir = format!("{}/.config", home_dir);
    let sudo_user = env::var("SUDO_USER").unwrap_or_else(|_| String::from(""));

    if !Path::new(&config_dir).exists() {
        fs::create_dir_all(&config_dir)?;
    }

    // If already installed, we overwrite.
    println!("Downloading Neovim configuration from GitHub...");
    run(
        "curl -fsSL https://github.com/spyral-ai/ignite/tarball/main -o /tmp/nvim-repo.tar.gz",
        true,
        None,
        false,
        0,
    )?;

    run("mkdir -p /tmp/nvim-extract", true, None, false, 0)?;
    run(
        "tar -xzf /tmp/nvim-repo.tar.gz -C /tmp/nvim-extract --strip-components=1",
        true,
        None,
        false,
        0,
    )?;

    // Copy only the nvim directory to the config location
    run(
        &format!("cp -r /tmp/nvim-extract/nvim {}", config_dir),
        true,
        None,
        false,
        0,
    )?;

    // Fix permissions if running as root for a regular user
    if !sudo_user.is_empty() {
        println!("Setting correct ownership for Neovim configuration...");
        run(
            &format!("chown -R {}:{} {}", sudo_user, sudo_user, config_dir),
            true,
            None,
            false,
            0,
        )?;

        // Also fix permissions for the vim-plug directory that will be created
        let plug_dir = format!("{}/.local/share/nvim", home_dir);
        if Path::new(&plug_dir).exists() {
            run(
                &format!("chown -R {}:{} {}", sudo_user, sudo_user, plug_dir),
                true,
                None,
                false,
                0,
            )?;
        }
    }

    // Clean up
    run("rm -f /tmp/nvim-repo.tar.gz", true, None, false, 0)?;
    run("rm -rf /tmp/nvim-extract", true, None, false, 0)?;

    // Install vim-plug
    run(
        &format!(
            "curl -fLo {}/.local/share/nvim/site/autoload/plug.vim --create-dirs \
            https://raw.githubusercontent.com/junegunn/vim-plug/master/plug.vim",
            home_dir
        ),
        true,
        None,
        false,
        0,
    )?;

    // After installing vim-plug, fix its permissions too
    if !sudo_user.is_empty() {
        let plug_dir = format!("{}/.local/share/nvim", home_dir);
        if Path::new(&plug_dir).exists() {
            run(
                &format!("chown -R {}:{} {}", sudo_user, sudo_user, plug_dir),
                true,
                None,
                false,
                0,
            )?;
        }
    }

    // Download and install neovim
    run(
        "curl -LO https://github.com/neovim/neovim/releases/latest/download/nvim-linux-x86_64.tar.gz",
        true,
        None,
        false,
        0,
    )?;

    run("rm -rf /opt/nvim", true, None, false, 0)?;
    run(
        "tar -C /opt -xzf nvim-linux-x86_64.tar.gz",
        true,
        None,
        false,
        0,
    )?;
    run("mv /opt/nvim-linux-x86_64 /opt/nvim", true, None, false, 0)?;

    // Create symlink to make nvim available in PATH
    run(
        "ln -sf /opt/nvim/bin/nvim /usr/local/bin/nvim",
        true,
        None,
        false,
        0,
    )?;

    if !sudo_user.is_empty() {
        println!(
            "Setting Neovim as the default Git editor for user {}...",
            sudo_user
        );
        run(
            &format!(
                "sudo -u {} git config --global core.editor \"nvim\"",
                sudo_user
            ),
            true,
            None,
            false,
            0,
        )?;
    } else {
        // If not running with sudo, configure for current user
        run(
            "git config --global core.editor \"nvim\"",
            true,
            None,
            false,
            0,
        )?;
    }

    // Clean up downloaded archive
    run("rm -f nvim-linux-x86_64.tar.gz", true, None, false, 0)?;

    println!("Neovim installation completed successfully!");
    Ok(())
}
