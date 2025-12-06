# Rusty Crab advanced programming project

## Environment setup
You can use [nix](https://nixos.org/explore/) to setup a developement environment, better with direnv.
Just run `nix develop` to enter a shell with all the requirements available, or `nix build` to build
the project without need to install anything else and pollute your sustem.
If rust-rover doesn't find the std just run `echo $RUST_SRC_PATH` and use this path when it asks to attach the
std manually.
