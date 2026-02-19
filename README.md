# Ignite

These scripts are useful for setting (igniting) up a new environment for development or production environment for Spyral.

The simplest way is to use one of the binary releases, which has the following commands:

```
Usage: ignite [OPTIONS] <COMMAND>

Commands:
  cuda         CUDA installation commands
  clang        Install Clang
  nvim         Install Neovim
  rust         Install Rust
  install-all  Install all components (Rust, CUDA, Clang, Neovim)
  help         Print this message or the help of the given subcommand(s)

Options:
  -c, --cloud-provider <CLOUD_PROVIDER>
          Cloud provider [default: gcp] [possible values: aws, gcp, azure]
  -h, --home-dir <HOME_DIR>
          The user home dir. If not specified, this will default to `/home/ubuntu'
  -h, --help
          Print help
  -V, --version
          Print version
```

Alternatively, if you want to compile the binary and then run it, you can use the `setup.sh` script which will install rust, and then you can build the binary with `cargo build --release` and then run the executable as above.

# Minimal setup

1) Run `source setup.sh` to install rust. You **may** need to run `source `~/.bashrc`.
2) Build the binary with `cargo build --release`.
3) Install the cuda driver with `./target/release/ignite --cloud-provider <aws|gcp> cuda instal-driver. This will require a reboot.
4) Install cuda with `./target/release/ignite cuda install-cuda.

Then you are good to go.

# Checking out repositories

You can check out the repositories using the `checkout-repos.sh` script, which will download all the development repositories to `~/Spyral`.

# Troubleshooting

If you have a cuda driver problem (i.e `nvidia-smi` gives an error, which is quite common) then run `ignite --cloud-provider <cloud_provider> cuda install-driver`.
