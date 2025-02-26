use crate::utils::run;
use std::io;

pub fn install_clang() -> io::Result<()> {
    println!("Installing Clang 17 and related tools...");

    // Add LLVM repository
    run(
        "echo \"deb http://apt.llvm.org/bookworm/ llvm-toolchain-bookworm-17 main\" >> /etc/apt/sources.list",
    )?;

    run(
        "echo \"deb-src http://apt.llvm.org/bookworm/ llvm-toolchain-bookworm-17 main\" >> /etc/apt/sources.list",
    )?;

    // Download and add LLVM GPG key
    run("wget -O - https://apt.llvm.org/llvm-snapshot.gpg.key")?;
    run("apt-key add -")?;

    // Update package lists and upgrade
    run("apt-get update")?;
    run("apt upgrade -y")?;

    // Install Clang 17 and related tools
    run("apt-get install -y clang-17 lld-17 lldb-17")?;

    println!("Clang 17 installation completed successfully!");
    Ok(())
}
