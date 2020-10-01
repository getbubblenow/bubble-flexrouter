# Building bubble-flexrouter on Windows

## Install MSVC build tools
If the MSVC build tools are not already installed, install them now.

Go to [https://visualstudio.microsoft.com/downloads/](https://visualstudio.microsoft.com/downloads/)

Download and run the installer. You don't need to install everything. Here's a screenshot showing which components
need to be installed:

<img src="img/win-build-tools-installer.png" alt="Screenshot of Windows Build Tools Installer" height="500"/>

[Screenshot of Windows Build Tools Installer](img/win-build-tools-installer.png)

## Install Rust
Go to [https://www.rust-lang.org/tools/install](https://www.rust-lang.org/tools/install)

Download and run the 64-bit version of `rustup-init.exe`

## Build it
Run `cargo build` to build the program

Run `cargo build --release` to build a release version
