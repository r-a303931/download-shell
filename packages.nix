{ pkgs, buildTools, buildEnvironment, craneLib, advisory-db, libnl, libiptc }:
let
  src = craneLib.cleanCargoSource ./.;

  commonArgs = buildEnvironment // {
    inherit src;
    strictDeps = true;

    nativeBuildInputs = buildTools;
    buildInputs = buildTools;

    RUSTFLAGS = "-Ctarget-feature=+crt-static";
  };

  gnuLinuxCargoArtifacts = craneLib.buildDepsOnly (commonArgs // {
    name = "dl-shell-deps-gnu-linux";
    cargoExtraArgs =
      "--target=x86_64-unknown-linux-gnu --locked -p sparse-server";
  });
  linuxCargoArtifacts = craneLib.buildDepsOnly (commonArgs // {
    name = "dl-shell-deps-linux";
    cargoExtraArgs =
      "--target=x86_64-unknown-linux-musl --locked -p sparse-unix-beacon -p sparse-unix-installer";
    RUSTFLAGS = "-Ctarget-feature=+crt-static";
  });

  download-shell = craneLib.buildPackage
    (commonArgs // { cargoArtifacts = linuxCargoArtifacts; });

  outputs = rec {
    packages = { default = download-shell; };
    checks = outputs.packages // {
      rs-fmt = craneLib.cargoFmt { inherit src; };
      rs-audit = craneLib.cargoAudit { inherit src advisory-db; };
      rs-toml-fmt = craneLib.taploFmt {
        src = pkgs.lib.sourceFilesBySuffices src [ ".toml" ];
      };
      rs-deny = craneLib.cargoDeny { inherit src; };
      rs-clippy = craneLib.cargoClippy (commonArgs // {
        cargoArtifacts = linuxCargoArtifacts;
        cargoClippyExtraArgs = "--all-targets -- --deny warnings";
      });
    };
  };
in outputs
