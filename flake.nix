# nix comments
{
  description = "Advanced programming 2025 library, RustyCrab";

  # Nixpkgs / NixOS version to use.
  inputs.nixpkgs.url = "nixpkgs/nixos-25.11";

  inputs.flake-utils.url = "github:numtide/flake-utils";
  
  inputs.self.submodules = true; # Use submodules

  outputs = { self, nixpkgs, flake-utils }:
    let
      version = "2.0.0";
      overlay = final: prev: { };
    in

    flake-utils.lib.eachDefaultSystem (system:
      let 
        pkgs = (nixpkgs.legacyPackages.${system}.extend overlay); 
        rust-toolchain = pkgs.symlinkJoin {
          name = "rust-toolchain";
          paths = with pkgs; [ rustc cargo rustPlatform.rustcSrc ];
        };
      in
      {

        packages = rec {
          default = rustycrab;
          rustycrab = pkgs.rustPlatform.buildRustPackage {
            pname = "rustycrab";
            src = pkgs.lib.cleanSource ./.;
            inherit version;

            nativeBuildInputs = with pkgs; [ 
              pkg-config 
              rustPlatform.bindgenHook
              ];
            buildInputs = with pkgs; [ 
              libxkbcommon
              vulkan-loader
              pipewire
              wayland 
              alsa-lib
              systemd
            ];
            
            cargoTestFlags = [ "--workspace" ];
            cargoLock = {
              lockFile = ./Cargo.lock;
              allowBuiltinFetchGit = true;
            };
          };
        };
        devShells = {
          default = pkgs.mkShell {
            inputsFrom = [ self.packages.${system}.rustycrab ];
            packages = with pkgs; [
              rust-toolchain 
              evcxr 
              (rustfmt.override { asNightly = true; })
              clippy
              (python3.withPackages (ps: with ps; [

              ]))
            ];
            RUST_BACKTRACE = 1;
            RUST_SRC_PATH = pkgs.rustPlatform.rustLibSrc;
            LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath [pkgs.libxkbcommon pkgs.vulkan-loader pkgs.pipewire];
            # LD_LIBRARY_PATH = pkgs.lib.makeLibraryPath self.packages.${system}.rustycrab.buildInputs;
            shellHook = ''
              source ${pkgs.rustPlatform.bindgenHook}/nix-support/setup-hook && populateBindgenEnv
            '';
          };
        };
      }
    );
}
