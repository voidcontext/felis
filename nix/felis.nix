{
  lib,
  stdenv,
  craneLib,
  libiconv,
  writeTextFile,
  ...
}: let
  src = ../.;

  helix-plugin = writeTextFile {
      name = "felis.scm";
      text = builtins.readFile ../felis.scm;
    };

  commonArgs = {
    inherit src;
    buildInputs = lib.optionals stdenv.isDarwin [
      libiconv
    ];
  };

  cargoArtifacts = craneLib.buildDepsOnly commonArgs;

  felis = craneLib.buildPackage (commonArgs
    // {
      doCheck = false;

      # # Shell completions
      # COMPLETIONS_TARGET = "target/";
      # nativeBuildInputs = [installShellFiles];
      # postInstall = ''
      #   installShellCompletion --bash target/felis.bash
      #   installShellCompletion --fish target/felis.fish
      #   installShellCompletion --zsh  target/_felis
      # '';

      passthru.checks = {
        inherit felis;

        felis-clippy = craneLib.cargoClippy (commonArgs
          // {
            inherit cargoArtifacts;
            cargoClippyExtraArgs = "--all-targets -- -Dwarnings -W clippy::pedantic -A clippy::missing-errors-doc -A clippy::missing-panics-doc";
          });

        felis-doc = craneLib.cargoDoc (commonArgs
          // {
            inherit cargoArtifacts;
          });

        # Check formatting
        felis-fmt = craneLib.cargoFmt {
          inherit src;
        };

        # # Audit dependencies
        # felis-audit = craneLib.cargoAudit {
        #   inherit src advisory-db;
        # };

        # # Audit licenses
        # felis-deny = craneLib.cargoDeny {
        #   inherit src;
        # };

        # Run tests with cargo-nextest
        # Consider setting `doCheck = false` on `felis` if you do not want
        # the tests to run twice
        felis-nextest = craneLib.cargoNextest (commonArgs
          // {
            inherit cargoArtifacts;
            partitions = 1;
            partitionType = "count";
            # skip integration tests
            cargoNextestExtraArgs = "-E 'not kind(test)'";
          });
      };

      passthru.helix-plugin = helix-plugin;
    });
in
  felis
