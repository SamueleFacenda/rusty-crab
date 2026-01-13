# nix comments
{
  description = "Advanced programming 2025 library, RustyCrab";

  # Nixpkgs / NixOS version to use.
  inputs.nixpkgs.url = "nixpkgs/nixos-25.11";

  inputs.flake-utils.url = "github:numtide/flake-utils";

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

        packages = let buildArgs = {
          pname = "rustycrab";
          src = pkgs.lib.cleanSource ./.;
          inherit version;

          buildInputs = with pkgs; [ ];
          
          cargoTestFlags = [ "--workspace" ];
        };
        in rec {
          default = rustycrab;
          rustycrab = pkgs.rustPlatform.buildRustPackage (buildArgs // {
            cargoLock = {
              lockFile = ./Cargo.lock;
              allowBuiltinFetchGit = true;
            };
          });
          rustycrab-nodeps = pkgs.rustPlatform.buildRustPackage (buildArgs // {
            cargoHash = pkgs.lib.fakeHash;
          });
        };
        devShells = {
          default = pkgs.mkShell {
            inputsFrom = [ self.packages.${system}.rustycrab-nodeps ];
            packages = with pkgs; [
              rust-toolchain 
              evcxr 
              rustfmt
              clippy
              (python3.withPackages (ps: with ps; [

              ]))
            ];
            RUST_BACKTRACE = 1;
            RUST_SRC_PATH = pkgs.rustPlatform.rustLibSrc;
          };
        };
      }
    );
}
