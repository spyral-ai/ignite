#!/bin/bash

git clone https://github.com/spyral-ai/scripts.git
cp -r scripts/nvim ~/.config/nvim

sudo apt install -y git-all
sudo apt install -y lua5.4
sudo apt install -y unzip
sudo apt install -y npm
curl -fsSL https://deno.land/install.sh | sh

# Install vim-plug
sh -c 'curl -fLo "${XDG_DATA_HOME:-$HOME/.local/share}"/nvim/site/autoload/plug.vim --create-dirs \
       https://raw.githubusercontent.com/junegunn/vim-plug/master/plug.vim'

# Download and install neovim
curl -LO https://github.com/neovim/neovim/releases/latest/download/nvim-linux64.tar.gz
sudo rm -rf /opt/nvim
sudo tar -C /opt -xzf nvim-linux64.tar.gz
sudo mv /opt/nvim-linux64 /opt/nvim
rm -rf nvim-linux64.tar.gz

# Make neovim the default editor for git
git config --global core.editor "nvim"

echo "PATH=/opt/nvim/bin:$PATH" >> ~/.bashrc
source ~/.bashrc
