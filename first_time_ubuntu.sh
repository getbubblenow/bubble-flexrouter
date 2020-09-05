#!/bin/bash

sudo apt install -y gcc curl libssl-dev pkg-config

curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

