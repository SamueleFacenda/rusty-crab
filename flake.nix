# nix comments
{
  description = "Advanced programming 2025 library, RustyCrab";

  # Nixpkgs / NixOS version to use.
  inputs.nixpkgs.url = "nixpkgs/nixos-25.11";

  inputs.flake-utils.url = "github:numtide/flake-utils";

  outputs = { self, nixpkgs, flake-utils }:
    let
      version = "0.1.0";
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

            buildInputs = with pkgs; [ ];
            
            cargoLock = {
              lockFile = ./Cargo.lock;
              allowBuiltinFetchGit = true;
            };
          };
        };
        devShells = {
          default = pkgs.mkShell {
            inputsFrom = [ self.packages.${system}.default ];
            packages = with pkgs; [
              rust-toolchain 
              evcxr 
              rustfmt
              clippy
              (python3.withPackages (ps: with ps; [

              ]))
              (rustPlatform.buildRustPackage rec {
                pname = "rust-obfuscator";
                version = "1.1.1";
              
                src = fetchFromGitHub {
                  owner = "dronavallipranav";
                  repo = "rust-obfuscator";
                  rev = "v${version}";
                  hash = "sha256-8cLB0jd+Hp8rptTK+Ji+8cc6w3/ZgUcpvj/MlwUCrlM=";
                };
              
                # buildAndTestSubdir = "obfuscator_cli";
                cargoHash = "sha256-ZXu0QFZ6mMtkWBCxvVgN5wVyn1f2GMaOkqA2lfPHTgA=";
              })
            ];
            RUST_BACKTRACE = 1;
            RUST_SRC_PATH = pkgs.rustPlatform.rustLibSrc;
          };
        };
      }
    );
}
