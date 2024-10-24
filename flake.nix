{
  description = "Dev";

  inputs = {
    crane = {
      url = "github:ipetkov/crane";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs = {
        nixpkgs.follows = "nixpkgs";
        flake-utils.follows = "flake-utils";
      };
    };
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    flake-utils.url = "github:numtide/flake-utils";
    flake-parts.url = "github:hercules-ci/flake-parts";
  };

  outputs = inputs @ {
    flake-parts,
    nixpkgs,
    flake-utils,
    crane,
    rust-overlay,
    ...
  }: let
    inherit (nixpkgs.lib) optional concatStringsSep;
    systems = flake-utils.lib.system;
    flake = flake-utils.lib.eachDefaultSystem (system: let
      overlays = [ (import rust-overlay) ];
      pkgs = import nixpkgs {
        inherit system overlays;
        config = { allowUnfree = true; };
      };

      aarch64DarwinExternalCargoCrates = concatStringsSep " " ["cargo-instruments@0.4.8"];

      defaultShellConf = {
        buildInputs = [
          pkgs.clang
          pkgs.libclang
          pkgs.pkg-config
          pkgs.libiconv
        ];

        nativeBuildInputs = with pkgs;
          [ 
            # Some issues while trying to cross compile it to aarch64-unknown-linux-gnu
            # https://github.com/NixOS/nixpkgs/issues/257258
            pkgsCross.aarch64-multiplatform.buildPackages.gcc
          ]
          ++ optional (system == systems.aarch64-darwin) [
            darwin.apple_sdk.frameworks.QuartzCore
            darwin.apple_sdk.frameworks.Foundation
            darwin.apple_sdk.frameworks.CoreFoundation
            darwin.apple_sdk.frameworks.CoreServices
            darwin.apple_sdk.frameworks.Security
            darwin.apple_sdk.frameworks.SystemConfiguration
          ]
          ++ optional (system != systems.aarch64-darwin) [
            pkgs.syslinux
          ];

        shellHook = ''
          # Add the GCC from the flake to the front of the PATH
          export PATH="${pkgs.pkgsCross.aarch64-multiplatform.buildPackages.gcc}/bin:$PATH"

          # Set CC environment variable to explicitly use this GCC
          export CC="${pkgs.pkgsCross.aarch64-multiplatform.buildPackages.gcc}/bin/gcc"

          # If you need to set other GCC-related variables:
          export CXX="${pkgs.pkgsCross.aarch64-multiplatform.buildPackages.gcc}/bin/g++"
          export LD="${pkgs.pkgsCross.aarch64-multiplatform.buildPackages.gcc}/bin/gcc"
        '';

      };
    in {
      devShells.default = pkgs.mkShell defaultShellConf;
    });
  in
    flake-parts.lib.mkFlake {inherit inputs;} {
      inherit flake;

      systems = flake-utils.lib.defaultSystems;

      perSystem = {
        config,
        system,
        ...
      }: {
        _module.args = {
          inherit crane;
          pkgs = import nixpkgs {
            inherit system;
            overlays = [(import rust-overlay)];
          };
        };
      };
    };
}

