# Scripts

A collection of useful scripts.

These scripts are useful for setting up a new environment for development of the Spyral libraries.

To setup a environment from scratch run the scripts in the following order:

```

# Optional, only required if usign neovim
sudo ./install-nvim.sh 

# Install rust, all repos are in rust.
sudo ./install-rust.sh

# Install cuda. These will require a reboot, and the driver install will need to be run twice.
# This will only work on debian based architectures.
sudo python3 install_cuda.py install_driver
sudo python3 install_cuda.py install_cuda

sudo ./install-clang.sh

# Checkout repos. You will need to generate a github autentication
# key and use that as the requested password
./checkout-repos.sh
```

# Troubleshooting

If you have a cuda driver problem (i.e `nvidia-smi` gives an error) then rerun `sudo python3 install_cuda.py install_driver` and reboot.
