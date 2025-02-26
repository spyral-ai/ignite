#!/bin/bash

sudo apt-get install -y git-all
sudo apt-get install -y lua5.4
sudo apt-get install -y unzip
sudo apt-get install -y npm
sudo apt-get install -y deno

# Copy nvim config
curl -o ~/.config/nvim --create-dirs https://github.com/spyral-ai/scripts/tree/main/nvim

# Install vim-plug
sh -c 'curl -fLo "${XDG_DATA_HOME:-$HOME/.local/share}"/nvim/site/autoload/plug.vim --create-dirs \
       https://raw.githubusercontent.com/junegunn/vim-plug/master/plug.vim'

# Download and install neovim
curl -LO https://github.com/neovim/neovim/releases/latest/download/nvim-linux-x86_64.tar.gz
sudo rm -rf /opt/nvim
sudo tar -C /opt -xzf nvim-linux-x86_64.tar.gz
sudo mv /opt/nvim-linux-x86_64 /opt/nvim

# Make neovim the default editor for git
git config --global core.editor "nvim"
