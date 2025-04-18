{
  description = "dl-shell";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";

    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    crane.url = "github:ipetkov/crane";
    flake-utils.url = "github:numtide/flake-utils";
    advisory-db = {
      url = "github:rustsec/advisory-db";
      flake = false;
    };

    libnl-src = {
      url = "git+https://github.com/thom311/libnl";
      flake = false;
    };
  };

  outputs =
    { self, nixpkgs, flake-utils, crane, rust-overlay, advisory-db, libnl-src }:
    flake-utils.lib.eachSystem [ "x86_64-linux" ] (system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [ (import rust-overlay) ];
        };

        system-libs = import ./system-libs.nix { inherit pkgs libnl-src; };
        inherit (system-libs) libnl libmnl libnftnl libiptc;

        buildTools = with pkgs; [ lld clang libclang libgcc mold glibc ];
        devShellTools = with pkgs; [
          rust-analyzer
          gdb
          taplo
          cargo-deny
          man-pages
          man-pages-posix
        ];

        craneLib = (crane.mkLib pkgs).overrideToolchain (p:
          p.rust-bin.nightly.latest.default.override {
            extensions = [ "rust-src" ];
            targets = [ "x86_64-unknown-linux-musl" ];
          });

        buildEnvironment = { DL_SHELL_LIBNL = libnl; };

        outputs = import ./packages.nix ({
          inherit pkgs buildTools buildEnvironment craneLib advisory-db;
        });
      in {
        packages = outputs.packages;
        checks = outputs.checks;

        devShells = {
          default = craneLib.devShell (buildEnvironment // {
            name = "dl-shell-default";

            packages = buildTools ++ devShellTools;
            nativeBuildTools = buildTools ++ (with pkgs; [ musl ]);
          });
        };
      });
}
