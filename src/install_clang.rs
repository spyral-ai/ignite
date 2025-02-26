use crate::utils::run;
use std::io;

pub fn install_clang() -> io::Result<()> {
    println!("Installing Clang 17 and related tools...");

    // Add LLVM repository
    run(
        "echo \"deb http://apt.llvm.org/bookworm/ llvm-toolchain-bookworm-17 main\" >> /etc/apt/sources.list",
        true,
        None,
        false,
        0,
    )?;

    run(
        "echo \"deb-src http://apt.llvm.org/bookworm/ llvm-toolchain-bookworm-17 main\" >> /etc/apt/sources.list",
        true,
        None,
        false,
        0,
    )?;

    // Download and add LLVM GPG key
    run(
        "wget -O - https://apt.llvm.org/llvm-snapshot.gpg.key | apt-key add -",
        true,
        None,
        false,
        0,
    )?;

    // Update package lists and upgrade
    run("apt-get update && apt upgrade -y", true, None, false, 0)?;

    // Install Clang 17 and related tools
    run(
        "apt-get install -y clang-17 lld-17 lldb-17",
        true,
        None,
        false,
        0,
    )?;

    println!("Clang 17 installation completed successfully!");
    Ok(())
}
