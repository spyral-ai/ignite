#!/usr/bin/env python3

# This is very much based off the the cuda toolkit and driver installation here:
# 
#   https://github.com/GoogleCloudPlatform/compute-gpu-installation
#
# It has been modified slighly to install a different (later) version of cuda and the toolkit, to
# only handle installation on Debian (since that's all that's used by Spyral), and to be slightly 
# simplified such that it is a single script.

import argparse
import os
import pathlib
import re
import shlex
import subprocess
import sys
import tempfile
import urllib.parse

CUDA_TOOLKIT_URL = "https://developer.download.nvidia.com/compute/cuda/12.5.0/local_installers/cuda_12.5.0_555.42.02_linux.run"
CUDA_TOOLKIT_CHECKSUM = "0bf587ce20c8e74b90701be56ae2c907"
CUDA_BIN_FOLDER = "/usr/local/cuda-12.5/bin"
CUDA_LIB_FOLDER = "/usr/local/cuda-12.5/lib64"
CUDA_PROFILE_FILENAME = pathlib.Path("/etc/profile.d/spyral_cuda_install.sh")

NVIDIA_PERSISTANCED_INSTALLER = (
    "/usr/share/doc/NVIDIA_GLX-1.0/samples/nvidia-persistenced-init.tar.bz2"
)

class RebootRequired(RuntimeError):
    pass

def run(command: str, check=True, input=None, silent=False, retries=0) -> subprocess.CompletedProcess:
    """ 
    Runs the provided command

    :param command: A command to execute, as a single stream.
    :param check: If true, will throw an exception on failure.
    :param input: Input for the executed command.
    :param silient: If true, will surpress the output.
    :param retries: The number of attempts at the command.

    :return: The CompletedProcess instance - the result of the command execution.
    """

    if not silent:
        print(f"Executing {command}", file=sys.stdout)

    stdout = []
    stderr = []
    proc = None
    try_count = 0

    while try_count <= retries:
        stdout.clear()
        stderr.clear()
        
        proc = subprocess.Popen(
            shlex.split(command),
            stderr=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stdin=subprocess.PIPE if input else None,
        )
        os.set_blocking(proc.stdout.fileno(), False)
        os.set_blocking(proc.stderr.fileno(), False)
        if input is not None:
            proc.stdin.wrote(input.encode())
            proc.stdin.close()

        def capture_comms():
            for line in proc.stdout.readlines():
                if not silent:
                    print(line.decode().strip(), file=sys.stdout)
                stdout.append(line.decode().strip())

            for line in proc.stderr.readlines():
                if not silent:
                    print(line.decode().strip(),file=sys.stderr)
                stderr.append(line.decode().strip())

        while proc.poll() is None:
            # While the process is running, capture the output
            capture_comms()
            try:
                proc.wait(0.1)
            except subprocess.TimeoutExpired:
                continue

        # When the process is finished, capture the remaining output
        capture_comms()

        if proc.returncode == 0:
            break
        else:
            try_count += 1
            continue

    if check and proc.returncode:
        raise subprocess.SubprocessError("Command exited with nonzero code")

    return subprocess.CompletedProcess(
        command, proc.returncode, stdout="\n".join(stdout), stderr="\n".join(stderr)
    )

def download_file(url: str, md5sum: str) -> pathlib.Path:
    """
    Downloads a file from the url and checks that the md5sum if the downloaded file is correct.
    """
    filename = urllib.parse.urlparse(url).path.split("/")[-1]
    file_path = pathlib.Path(filename)

    if not file_path.exists():
        run(f"curl -fSsL -O {url}")

    checksum = run(f"md5sum {file_path}").stdout.strip().split()[0]
    if checksum != md5sum:
        raise RuntimeError(
            f"The installer file checksum does not match. Won't continue installation."
            f"Try deleting {file_path.absolute()} and trying again."
        )
        
    return file_path
    
def get_kernel_version():
    run("uname -r", silent=True).stdout

def install_dependencies_debian():
    """
    Installs the dependecies required to install cuda or the driver.
    """

    KERNEL_IMAGE_PACKAGE = "linux-image-{version}"
    KERNEL_VERSION_FORMAT = "{major}.{minor}.{patch}-{micro}-cloud-amd64"
    KERNEL_HEADERS_PACKAGE = "linux-headers-{version}"
    KERNEL_PACKAGE_REGEX = r"linux-image-{major}.{minor}.([\d]+)-([\d]+)-cloud-amd64"

    run("apt-get update")

    kernel_version = get_kernel_version()
    major, minor, *_ = kernel_version.split(".")
    kernel_package_regex = re.compile(
        KERNEL_PACKAGE_REGEX.format(major=major, minor=minor)
    )

    # Find the newest version of kernel to update to, but staying with the same major version
    packages = run("apt-cache search linux-image").stdout
    patch, micro = max(kernel_package_regex.findall(packages))

    wanted_kernel_version = self.KERNEL_VERSION_FORMAT.format(
        major=major, minor=minor, patch=patch, micro=micro
    )
    wanted_kernel_package = self.KERNEL_IMAGE_PACKAGE.format(
        version=wanted_kernel_version
    )
    wanted_kernel_headers = self.KERNEL_HEADERS_PACKAGE.format(
        version=wanted_kernel_version
    )

    run(
        f"apt-get install -y make gcc {wanted_kernel_package} {wanted_kernel_headers} "
        f"software-properties-common pciutils gcc make dkms"
    )


    raise RebootRequired

def lock_kernel_updates_debian():
    """
    Marks kernel related packages, so they don't get auto-updated. This would cause the driver to stop working.
    """

    print("Locking kernel updates ...")

    kernel_version = get_kernel_version()
    run(
        f"apt-mark hold "
        f"linux-image-{self.kernel_version} "
        f"linux-headers-{self.kernel_version} "
        f"linux-image-cloud-amd64 "
        f"linux-headers-cloud-amd64"
    )

def unlock_kernel_updates_debian():
    """
    Allows the kernel related packages to be upgraded.
    """

    print("Unlocking kernel updates...")

    kernel_version = get_kernel_version()
    run(
        f"apt-mark unhold "
        f"linux-image-{kernel_version} "
        f"linux-headers-{kernel_version} "
        f"linux-image-cloud-amd64 "
        f"linux-headers-cloud-amd64"
    )

def reboot():
    """
    Reboots the system.
    """

    print("The system needs to be rebooted to complete the installation process. ")
    print("The process will be continued after the reboot.")

    run("reboot now")
    sys.exit(0)

 
def download_cuda_toolkit_installer() -> pathlib.Path:
    print("Downloading CUDA installation toolkit ...")
    download_file(CUDA_TOOLKIT_URL, CUDA_TOOLKIT_CHECKSUM)

def install_driver():
    """
    Downloads the installation package and installs the driver. It also handles installation of
    driver prerequisites and will trigger a reboot on first run, when those prerequisites are installed.

    On second run, it will proceed to download proper installer and install the driver. When it's done, `nvidia-smi`
    should be available in the system and the drivers are installed.

    It also triggers kernel packages lock in the system, so the driver is not broken by auto-updates.
    """

    try:
        install_dependencies_debian()
    except RebootRequired:
        reboot()

    print("Installing GPU drivers ...")

    installer_path = download_cuda_toolkit_installer() 
    run(f"sh {installer_path} --silent --driver", check=True)

    if verify_driver(verbose=True):
        lock_kernel_updates_debian()
    else:
        print("Something went wrong with driver installation, installation failed")

def uninstall_driver():
    """
    Uses the Nvidia installers to execute driver uninstallation. It will also unlock the kernel updates in the
    system.
    """
    if not verify_driver():
        print("GPU driver not found.")
        return

    with tempfile.TemporaryDirectory() as temp_dir:
        installer_path = download_cuda_toolkit_installer()
        print("Extracting NVIDIA driver installer, to complete uninstallation...")
        run(f"sh {installer_path} --extract={temp_dir}", check=True)

        installer_path = pathlib.Path(f"{temp_dir}/NVIDIA-Linux-x86_64-555.42.02.run")

        print("Starting uninstallation...")
        run(f"sh {installer_path} -s --uninstall", check=True)
        print("Uninstallation completed!")
        unlock_kernel_updates_debian()

def verify_driver(verbose:bool = False) -> bool:
    """
    Checks if the driver is already installed by calling the `nvidia-smi` binary.
    If it's available and doesn't produce errors, that means the driver is already installed.
    """

    process = run("which nvidia-smi", check=False, silent=True)
    if process.returncode != 0:
        if verbose:
            print("Couldn't find nvidia-smi, the driver is not installed.")
        return False

    process2 = run("nvidia-smi -L", check=False, silent=True)
    success = process2.returncode == 0 and "UUID" in process2.stdout
    if verbose:
        print(f"nvidia-smi -L output: {process2.stdout} {process2.stderr}")
        
    return success

def configure_persistanced_service():
    """
    Configures the nvidia-persistenced daemon to auto-start. It creates a service to be 
    controlled using `systemctl`.
    """
    if not pathlib.Path("/usr/bin/nvidia-persistenced").exists():
        return

    if not pathlib.Path(NVIDIA_PERSISTANCED_INSTALLER).exists():
        return

    with tempfile.TemporaryDirectory() as temp_dir:
        shutil.copy(NVIDIA_PERSISTANCED_INSTALLER, temp_dir + "/installer.tar.bz2")
        with chdir(temp_dir):
            run("tar -xf installer.tar.bz2", silent=True)
            print("Executing nvidia-persistenced installer...")
            run("sh nvidia-persistenced-init/install.sh", check=True)

def cuda_postinstallation_actions(self):
    """
    Perform required and suggested post-installation actions:
    
    * sets environment variables
    * makes persistent changes to environment variables
    * configure nvidia-persistanced to auto-start (if exists)

    For more info, see: 
        https://docs.nvidia.com/cuda/cuda-installation-guide-linux/index.html#post-installation-actions
    """

    os.environ["PATH"] = f"{CUDA_BIN_FOLDER}:{os.environ['PATH']}"
    if "LD_LIBRARY_PATH" in os.environ:
        os.environ["LD_LIBRARY_PATH"] = (
            f"{CUDA_LIB_FOLDER}:{os.environ['LD_LIBRARY_PATH']}"
        )
    else:
        os.environ["LD_LIBRARY_PATH"] = CUDA_LIB_FOLDER

    with CUDA_PROFILE_FILENAME.open("w") as profile:
        profile.write(
            "# Configuring CUDA toolkit. File created by Spyral CUDA installation manager.\n"
        )
        profile.write("export PATH=" + CUDA_BIN_FOLDER + "${PATH:+:${PATH}}\n")
        profile.write(
            "export LD_LIBRARY_PATH="
            + CUDA_LIB_FOLDER
            + "${LD_LIBRARY_PATH:+:${LD_LIBRARY_PATH}}\n"
        )

        configure_persistanced_service()

def install_cuda_inner():
    """
    Installs the cuda toolkit and executes post-installation configuration in 
    the operating system, to make it available for all users.
    """

    if not verify_driver():
        print(
            "CUDA installation requires GPU driver to be installed first. "
            "Attempting to install GPU driver now."
        )
        install_driver()

    installer_path = download_cuda_toolkit_installer()

    print("Installing CUDA toolkit...")
    run(f"sh {installer_path} --silent --toolkit", check=True)
    print("CUDA toolkit installation completed!")

    print("Executing post-installation actions...")
    cuda_postinstallation_actions()
    print("CUDA post-installation actions completed!")

    raise RebootRequired

def install_cuda():
    """
    Installs the cuda toolkit and executes post-installation configuration in 
    the operating system, to make it available for all users.
    """
    try:
        install_cuda_inner()
    except RebootRequired:
        reboot()

def parse_args():
    parser = argparse.ArgumentParser(
        description="Manage GPU drivers and CUDA toolkit installation."
    )
    parser.add_argument(
        "command",
        choices=[
            "install_driver",
            "install_cuda",
            "uninstall_driver",
            "verify_driver",
        ],
        help="Install GPU driver or CUDA Toolkit.",
    )

    return parser.parse_args()


def main():
    if os.geteuid() != 0:
        print("This script needs to be run with root privileges!")
        sys.exit(1)

    args = parse_args()

    if args.command == "install_driver":
        install_driver()
    elif args.command == "verify_driver":
        if verify_driver(verbose=True):
            sys.exit(0)
        else:
            sys.exit(1)
    elif args.command == "uninstall_driver":
        uninstall_driver()
    elif args.command == "install_cuda":
        install_cuda()
    else:
        print(f"Invalid command {args.command}")
        sys.exit(1)

if __name__ == "__main__":
    main()
