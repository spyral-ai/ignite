#!/bin/bash

sudo apt-get update
sudo apt-get install -y build-essential
sudo apt-get install -y clang

curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
source $HOME/.cargo.env
rustup toolchain install nightly
rustup default nightly
rustup component add rust-analyzer

curl -L --proto '=https' --tlsv1.2 -sSf https://raw.githubusercontent.com/cargo-bins/cargo-binstall/main/install-from-binstall-release.sh | sh -s -- -y

source ~/.bashrc

cargo binstall cargo-nextest --secure
