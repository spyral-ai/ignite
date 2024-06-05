#!/bin/bash

# Cuda
wget https://developer.download.nvidia.com/compute/cuda/12.5.0/local_installers/cuda-repo-debian12-12-5-local_12.5.0-555.42.02-1_amd64.deb
sudo dpkg -i cuda-repo-debian12-12-5-local_12.5.0-555.42.02-1_amd64.deb
sudo cp /var/cuda-repo-debian12-12-5-local/cuda-*-keyring.gpg /usr/share/keyrings/
sudo add-apt-repository contrib
sudo apt-get update
sudo apt-get -y install cuda-toolkit-12-5
echo "PATH=/usr/local/cuda/bin:$PATH" >> ~/.bashrc
source ~/.bashrc

git clone https://github.com/GoogleCloudPlatform/compute-gpu-installation.git
cd compute-gpu-installation/linux
sudo python3 install_gpu_driver.py
