{ pkgs, buildTools, buildEnvironment, craneLib, advisory-db }:
let
  src = craneLib.cleanCargoSource ./.;

  commonArgs = buildEnvironment // {
    inherit src;
    strictDeps = true;

    nativeBuildInputs = buildTools ++ (with pkgs; [ musl ]);
    buildInputs = buildTools;

    #RUSTFLAGS = "-Clink-arg=-fuse-ld=mold";

    cargoExtraArgs = "--target=x86_64-unknown-linux-musl --locked";
  };

  cargoArtifacts =
    craneLib.buildDepsOnly (commonArgs // { name = "dl-shell-deps-linux"; });

  download-shell = craneLib.buildPackage (commonArgs // {
    doCheck = false;
    cargoArtifacts = cargoArtifacts;
  });

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
        cargoClippyExtraArgs =
          "--target=x86_64-unknown-linux-musl -- --deny warnings";
      });
    };
  };
in outputs
