{ pkgs, buildTools, buildEnvironment, craneLib, advisory-db, libnl, libiptc }:
let
  src = craneLib.cleanCargoSource ./.;

  commonArgs = buildEnvironment // {
    inherit src;
    strictDeps = true;

    nativeBuildInputs = buildTools ++ (with pkgs; [ musl ]);
    buildInputs = buildTools;

    RUSTFLAGS = "-Clink-arg=-fuse-ld=mold";
  };

  cargoArtifacts = craneLib.buildDepsOnly (commonArgs // {
    name = "dl-shell-deps-linux";
    cargoExtraArgs = "--target=x86_64-unknown-linux-musl --locked";
  });

  download-shell =
    craneLib.buildPackage (commonArgs // { cargoArtifacts = cargoArtifacts; });

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
        cargoArtifacts = cargoArtifacts;
        cargoClippyExtraArgs = "--all-targets -- --deny warnings";
      });
    };
  };
in outputs
