#!/bin/bash

git config --global credential.helper store

mkdir -p ~/Spyral && cd ~/Spyral
git clone https://github.com/spyral-ai/flame.git
git clone https://github.com/spyral-ai/spyral.git
git clone https://github.com/spyral-ai/cuda-rs.git
git clone https://github.com/spyral-ai/rocm-rs.git
git clone https://github.com/spyral-ai/zync.git
