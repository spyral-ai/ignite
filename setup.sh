#!/bin/bash

sudo apt-get update
sudo apt-get install -y build-essential

curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
source $HOME/.cargo.env
rustup toolchain install nightly
rustup default nightly
rustup component add rust-analyzer

source ~/.bashrc
