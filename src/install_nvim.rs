use crate::utils::run;
use std::{env, fs, io, path::Path};

pub fn install_nvim() -> io::Result<()> {
    println!("Installing Neovim and dependencies...");

    // Install dependencies
    run("apt-get install -y git-all", true, None, false, 0)?;
    run("apt-get install -y lua5.4", true, None, false, 0)?;
    run("apt-get install -y unzip", true, None, false, 0)?;
    run("apt-get install -y npm", true, None, false, 0)?;
    run("apt-get install -y deno", true, None, false, 0)?;

    // Create config directory if it doesn't exist
    let home_dir = env::var("HOME").unwrap_or_else(|_| String::from("/root"));
    let config_dir = format!("{}/.config/nvim", home_dir);

    if !Path::new(&config_dir).exists() {
        fs::create_dir_all(&config_dir)?;

        // Copy nvim config from local directory instead of downloading
        let source_dir = "nvim";
        if Path::new(source_dir).exists() {
            println!("Copying Neovim configuration from local directory...");
            run(
                &format!("cp -r {}/* {}", source_dir, config_dir),
                true,
                None,
                false,
                0,
            )?;
        } else {
            println!("Local nvim directory not found, downloading from GitHub...");
            // Fallback to downloading if local directory doesn't exist
            run(
                "curl -fsSL https://github.com/spyral-ai/scripts/archive/refs/heads/main.zip -o /tmp/nvim-config.zip",
                true,
                None,
                false,
                0,
            )?;

            run(
                "unzip -q /tmp/nvim-config.zip -d /tmp",
                true,
                None,
                false,
                0,
            )?;
            run(
                &format!("cp -r /tmp/scripts-main/nvim/* {}", config_dir),
                true,
                None,
                false,
                0,
            )?;

            // Clean up
            run("rm -f /tmp/nvim-config.zip", true, None, false, 0)?;
            run("rm -rf /tmp/scripts-main", true, None, false, 0)?;
        }
    }

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

    // Make neovim the default editor for git
    run(
        "git config --global core.editor \"nvim\"",
        true,
        None,
        false,
        0,
    )?;

    // Clean up downloaded archive
    run("rm -f nvim-linux-x86_64.tar.gz", true, None, false, 0)?;

    println!("Neovim installation completed successfully!");
    Ok(())
}
