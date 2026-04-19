use crate::utils::{run_cmd, CommandOptions};
use std::{env, fs, io, path::Path};

pub fn install_nvim(home_dir: String) -> io::Result<()> {
    println!("Installing Neovim and dependencies...");

    // Install dependencies
    run_cmd(
        "apt-get",
        ["install", "-y", "git-all"],
        CommandOptions::default(),
    )?;
    run_cmd(
        "apt-get",
        ["install", "-y", "lua5.4"],
        CommandOptions::default(),
    )?;
    run_cmd(
        "apt-get",
        ["install", "-y", "unzip"],
        CommandOptions::default(),
    )?;
    run_cmd(
        "apt-get",
        ["install", "-y", "npm"],
        CommandOptions::default(),
    )?;

    run_cmd(
        "curl",
        [
            "-fsSL",
            "https://deno.land/install.sh",
            "-o",
            "/tmp/deno_install.sh",
        ],
        CommandOptions::default(),
    )?;
    run_cmd("sh", ["/tmp/deno_install.sh"], CommandOptions::default())?;

    // Create config directory if it doesn't exist
    println!("Installing neovim with home dir: {home_dir}");
    let config_dir = format!("{}/.config", home_dir);
    let sudo_user = env::var("SUDO_USER").unwrap_or_else(|_| String::from(""));

    if !Path::new(&config_dir).exists() {
        fs::create_dir_all(&config_dir)?;
    }

    // If already installed, we overwrite.
    println!("Downloading Neovim configuration from GitHub...");
    run_cmd(
        "curl",
        [
            "-fsSL",
            "https://github.com/spyral-ai/ignite/tarball/main",
            "-o",
            "/tmp/nvim-repo.tar.gz",
        ],
        CommandOptions::default(),
    )?;

    run_cmd(
        "mkdir",
        ["-p", "/tmp/nvim-extract"],
        CommandOptions::default(),
    )?;
    run_cmd(
        "tar",
        [
            "-xzf",
            "/tmp/nvim-repo.tar.gz",
            "-C",
            "/tmp/nvim-extract",
            "--strip-components=1",
        ],
        CommandOptions::default(),
    )?;

    // Copy only the nvim directory to the config location
    println!("Copying nvim from /tmp/nvim-extract to {config_dir}");
    run_cmd(
        "cp",
        ["-r", "/tmp/nvim-extract/nvim", config_dir.as_str()],
        CommandOptions::default(),
    )?;

    // Fix permissions if running as root for a regular user
    if !sudo_user.is_empty() {
        println!("Setting correct ownership for Neovim configuration...");
        let owner = format!("{sudo_user}:{sudo_user}");
        run_cmd(
            "chown",
            ["-R", owner.as_str(), config_dir.as_str()],
            CommandOptions::default(),
        )?;

        // Also fix permissions for the vim-plug directory that will be created
        let plug_dir = format!("{home_dir}/.local/share/nvim");
        if Path::new(&plug_dir).exists() {
            run_cmd(
                "chown",
                ["-R", owner.as_str(), plug_dir.as_str()],
                CommandOptions::default(),
            )?;
        }
    }

    // Clean up
    run_cmd(
        "rm",
        ["-f", "/tmp/nvim-repo.tar.gz"],
        CommandOptions::default(),
    )?;
    run_cmd(
        "rm",
        ["-rf", "/tmp/nvim-extract"],
        CommandOptions::default(),
    )?;

    // Install vim-plug
    let plug_path = format!("{home_dir}/.local/share/nvim/site/autoload/plug.vim");
    run_cmd(
        "curl",
        [
            "-fLo",
            plug_path.as_str(),
            "--create-dirs",
            "https://raw.githubusercontent.com/junegunn/vim-plug/master/plug.vim",
        ],
        CommandOptions::default(),
    )?;

    // After installing vim-plug, fix its permissions too
    if !sudo_user.is_empty() {
        let plug_dir = format!("{home_dir}/.local/share/nvim");
        if Path::new(&plug_dir).exists() {
            let owner = format!("{sudo_user}:{sudo_user}");
            run_cmd(
                "chown",
                ["-R", owner.as_str(), plug_dir.as_str()],
                CommandOptions::default(),
            )?;
        }
    }

    // Download and install neovim
    run_cmd(
        "curl",
        [
            "-LO",
            "https://github.com/neovim/neovim/releases/latest/download/nvim-linux-x86_64.tar.gz",
        ],
        CommandOptions::default(),
    )?;

    run_cmd("rm", ["-rf", "/opt/nvim"], CommandOptions::default())?;
    run_cmd(
        "tar",
        ["-C", "/opt", "-xzf", "nvim-linux-x86_64.tar.gz"],
        CommandOptions::default(),
    )?;
    run_cmd(
        "mv",
        ["/opt/nvim-linux-x86_64", "/opt/nvim"],
        CommandOptions::default(),
    )?;

    // Create symlink to make nvim available in PATH
    run_cmd(
        "ln",
        ["-sf", "/opt/nvim/bin/nvim", "/usr/local/bin/nvim"],
        CommandOptions::default(),
    )?;

    if !sudo_user.is_empty() {
        println!(
            "Setting Neovim as the default Git editor for user {}...",
            sudo_user
        );
        run_cmd(
            "sudo",
            [
                "-u",
                sudo_user.as_str(),
                "git",
                "config",
                "--global",
                "core.editor",
                "nvim",
            ],
            CommandOptions::default(),
        )?;
    } else {
        // If not running with sudo, configure for current user
        run_cmd(
            "git",
            ["config", "--global", "core.editor", "nvim"],
            CommandOptions::default(),
        )?;
    }

    // Clean up downloaded archive
    run_cmd(
        "rm",
        ["-f", "nvim-linux-x86_64.tar.gz"],
        CommandOptions::default(),
    )?;

    println!("Neovim installation completed successfully!");
    Ok(())
}
