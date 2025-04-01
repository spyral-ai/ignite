use crate::utils::run;
use std::io;

pub fn install_clang(version: usize) -> io::Result<()> {
    println!("Installing Clang {version} and related tools...");

    run("wget https://apt.llvm.org/llvm.sh")?;
    run("chmod +x llvm.sh")?;
    run(&format!("./llvm.sh {version}"))?;
    run(&format!(
        "apt install -y clang-{version} lld-{version} lldb-{version}"
    ))?;

    println!("Clang {version} installation completed successfully!");
    Ok(())
}
