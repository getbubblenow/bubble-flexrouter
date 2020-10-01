# Building bubble-flexrouter
These instructions work for Linux and Mac OS X.

For Windows, see the [Windows build instructions](BUILD-windows.md)

## For Ubuntu users
Run:

```shell script
first_time_ubuntu.sh
```

This will install the required `apt` packages and Rust.

This command will probably work on any Debian-based system.

## For Mac OS and other Linux distributions
Look in `first_time_ubuntu` -- those are the packages that you'll need to install.

Once those are installed, you can install Rust:

```shell script
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

## Build it
Run `cargo build` to build the program

Run `cargo build --release` to build a release version
