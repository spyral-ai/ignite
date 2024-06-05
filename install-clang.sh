#!/bin/bash

sudo echo "deb http://apt.llvm.org/bookworm/ llvm-toolchain-bookworm-17 main" >> /etc/apt/sources.list
sudo echo "deb-src http://apt.llvm.org/bookworm/ llvm-toolchain-bookworm-17 main" >> /etc/apt/sources.list
wget -O - https://apt.llvm.org/llvm-snapshot.gpg.key | sudo apt-key add -
sudo apt-get update && sudo apt upgrade
sudo apt-get install -y clang-17 lld-17 lldb-17
