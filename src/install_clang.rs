use crate::utils::{run_cmd, CommandOptions};
use std::io;

pub fn install_clang(version: usize) -> io::Result<()> {
    println!("Installing Clang {version} and related tools...");

    run_cmd(
        "wget",
        ["https://apt.llvm.org/llvm.sh"],
        CommandOptions::default(),
    )?;
    run_cmd("chmod", ["+x", "llvm.sh"], CommandOptions::default())?;
    let version_string = version.to_string();
    run_cmd(
        "./llvm.sh",
        [version_string.as_str()],
        CommandOptions::default(),
    )?;
    let clang = format!("clang-{version}");
    let lld = format!("lld-{version}");
    let lldb = format!("lldb-{version}");
    run_cmd(
        "apt",
        ["install", "-y", clang.as_str(), lld.as_str(), lldb.as_str()],
        CommandOptions::default(),
    )?;

    println!("Clang {version} installation completed successfully!");
    Ok(())
}
