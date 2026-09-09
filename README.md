# Ignite

These scripts are useful for setting (igniting) up a new environment for development or production environment for Spyral.

The simplest way is to use one of the binary releases, which has the following commands:

```
Usage: ignite [OPTIONS] <COMMAND>

Commands:
  cuda         CUDA installation commands
  nvim         Install Neovim
  rust         Install Rust
  setup-env    Configure Moonlite and NCCL environment variables for a user shell
  mount        Persistently mount a block device at a mountpoint
  install-all  Install all components (Rust and CUDA)
  help         Print this message or the help of the given subcommand(s)

Options:
  -c, --cloud-provider <CLOUD_PROVIDER>
          Cloud provider [default: gcp] [possible values: aws, gcp, azure]
      --home-dir <HOME_DIR>
          The user home dir. If not specified, this will default to `/home/ubuntu'
  -h, --help
          Print help
  -V, --version
          Print version
```

Alternatively, if you want to compile the binary and then run it, you can use the `setup.sh` script which will install rust, and then you can build the binary with `cargo build --release` and then run the executable as above.

# Minimal setup

1) Run `source setup.sh` to install rust and some other deps. You **may** need to run `source `~/.bashrc`.
2) Build the binary with `cargo build --release`.
3) Install the CUDA driver with `sudo ./target/release/ignite cuda install-driver`.
4) Install the driver and CUDA toolkit together with `sudo ./target/release/ignite cuda install-cuda`.
5) Install nccl with `./target/release/ignite cuda install-nccl`.
6) Optionally, mount a data drive with `./target/release/ignite mount <device> <mountpoint> --provision`, so for example, `./target/release/ignite mount nvme0n2 /mnt/disks/public --provision`.
7) Configure the environment with `./target/release/ignite setup-env --user=<username> --moonlite-dir=/mnt/disks/public/moonlite`.

Then you are good to go.

# Reproducible CUDA hosts

CUDA commands default to CUDA 13.3.1, the newest release currently supported by Ignite. Pass `--version` to select another supported release.

On Ubuntu 24.04, Ignite installs and boots a provider-specific kernel selected for the requested CUDA release. It installs exact kernel package versions, pins the image, headers, and modules packages, prevents `unattended-upgrades` from advancing the provider kernel packages, and configures GRUB to keep booting the selected kernel.

When the selected kernel requires a reboot, Ignite installs a one-shot systemd unit and continues the requested command automatically after boot. Its output is available through `journalctl -u ignite-cuda-resume.service`. Ignite removes the unit after the installation succeeds.

| CUDA | NVIDIA driver | AWS kernel | Azure kernel | GCP kernel |
|---|---|---|---|---|
| 12.5 | 555.42.02 | 6.8.0-1008-aws | 6.8.0-1007-azure | 6.8.0-1007-gcp |
| 12.6 | 560.28.03 | 6.8.0-1008-aws | 6.8.0-1007-azure | 6.8.0-1007-gcp |
| 12.8 | 570.86.10 | 6.8.0-1008-aws | 6.8.0-1007-azure | 6.8.0-1007-gcp |
| 13.0.1 | 580.82.07 | 6.17.0-1020-aws | 6.17.0-1022-azure | 6.17.0-1022-gcp |
| 13.1.1 | 590.48.01 | 6.17.0-1020-aws | 6.17.0-1022-azure | 6.17.0-1022-gcp |
| 13.2.1 | 595.58.03 | 6.17.0-1020-aws | 6.17.0-1022-azure | 6.17.0-1022-gcp |
| 13.3.1 | 610.43.02 | 7.0.0-1012-aws | 7.0.0-1008-azure | 7.0.0-1011-gcp |

Pinned CUDA installation supports Ubuntu 24.04 on AWS, Azure, and GCP. Ignite reports an error for other distributions until an exact kernel tuple has been added and validated for them.

# Persistent mounts

You can persistently mount an existing filesystem with:

```bash
./target/debug/ignite mount <device> <mountpoint>
```

If the device is blank and you want `ignite` to format the whole disk first, use `--provision`. The default filesystem for provisioning is `ext4`.

Example:

```bash
./target/debug/ignite mount nvme0n2 /mnt/disks/public --provision
```

This command:

1. Creates the mountpoint if it does not exist.
2. Formats the whole device when `--provision` is set and the device does not already contain a filesystem.
3. Looks up the filesystem UUID for the device.
4. Writes or updates the corresponding `/etc/fstab` entry.
5. Mounts the filesystem immediately.
6. Sets the mounted directory owner to the invoking `sudo` user by default.

If the device is blank and `--provision` is omitted, `ignite` will fail rather than formatting it implicitly.

# NCCL

You can build and install NCCL into `/opt/nccl` with:

```bash
./target/debug/ignite cuda install-nccl
```

You can also choose a different destination:

```bash
./target/debug/ignite cuda install-nccl --install-dir=/opt/nccl
```

If you want `ignite` to write `/etc/profile.d/spyral_nccl.sh`, add `--write-profile`.

This command builds the current NCCL source release against the detected CUDA installation, copies the resulting `include/` and `lib/` directories into the requested install directory, and prints the environment variables you can add to `~/.bashrc` yourself.

# Setup Env

You can configure the Moonlite and NCCL shell environment for a specific user with:

```bash
sudo ./target/debug/ignite setup-env --user=robclucas --moonlite-dir=/mnt/disks/public/moonlite --venv-dir=/mnt/disks/public/venvs/spyral-venv
```

This command:

1. Creates `<moonlite-dir>/kernels` and `<moonlite-dir>/models`.
2. Detects `NCCL_ROOT` under `/opt` first, then `/usr/local`.
3. Detects the CUDA installation root used for `CUDA_ROOT`.
4. Optionally adds `source <venv-dir>/bin/activate` to the managed `.bashrc` block when `--venv-dir` is provided.
5. Writes or updates a `ignite setup-env` block in `/home/robclucas/.bashrc`.
6. Sets ownership of the created directories and `.bashrc` back to the target user.

With the example above, the generated paths are:

```text
MOONLITE_MODELS_PATH=/mnt/disks/public/moonlite/models
MOONLITE_KERNEL_OUTPUT_DIR=/mnt/disks/public/moonlite/kernels
MOONLITE_KERNELS=/home/robclucas/Spyral/moonlite/crates/moonlite_compute/src/kernels
```

# Checking out repositories

You can check out the repositories using the `checkout-repos.sh` script, which will download all the development repositories to `~/Spyral`.

# Troubleshooting

If you have a cuda driver problem (i.e `nvidia-smi` gives an error, which is quite common) then run `ignite --cloud-provider <cloud_provider> cuda install-driver`.
