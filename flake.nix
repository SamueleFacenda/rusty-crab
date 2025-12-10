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
                pname = "rustfuscator";
                version = "0.2.10";
              
                src = fetchFromGitHub {
                  owner = "GianIac";
                  repo = "rustfuscator";
                  rev = "v${version}";
                  hash = "sha256-AG1PW5fN7TjxfCTVwig95iTRfpzHe38dNdwcXVZxHpg=";
                };
              
                buildAndTestSubdir = "obfuscator_cli";
                cargoHash = "sha256-UT3Rkq5r56Hy88zdlm21iNTyeqEa2mqU5b7oEeCeKE0=";
              })
            ];
            RUST_BACKTRACE = 1;
            RUST_SRC_PATH = pkgs.rustPlatform.rustLibSrc;
          };
        };
      }
    );
}
