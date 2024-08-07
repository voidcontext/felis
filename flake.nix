{
  inputs.nixpkgs.url = "nixpkgs/release-23.05";
  inputs.flake-utils.url = "github:numtide/flake-utils";

  inputs.crane = {
    url = "github:ipetkov/crane";
    inputs.nixpkgs.follows = "nixpkgs";
  };
  inputs.nix-rust-utils.url = "git+https://git.vdx.hu/voidcontext/nix-rust-utils.git?ref=refs/tags/v0.10.0";
  inputs.nix-rust-utils.inputs.nixpkgs.follows = "nixpkgs";
  inputs.nix-rust-utils.inputs.crane.follows = "crane";

  inputs.rust-overlay.url = "github:oxalica/rust-overlay";

  outputs = {
    nixpkgs,
    flake-utils,
    nix-rust-utils,
    rust-overlay,
    ...
  }: let
    mkNru = pkgs:
      nix-rust-utils.mkLib {
        inherit pkgs;
        toolchain = pkgs.rust-bin.stable.latest.default;
      };
    mkPlugin = pkgs:
      pkgs.writeTextFile {
        name = "felis.scm";
        text = builtins.readFile ./felis.scm;
      };
    mkFelis = pkgs: let
      nru = mkNru pkgs;

      commonArgs = {
        src = ./.;
        buildInputs = pkgs.lib.optionals pkgs.stdenv.isDarwin [
          pkgs.libiconv
        ];
      };
    in rec {
      crate = nru.mkCrate (commonArgs
        // {
          doCheck = true;

          # # Shell completions
          # COMPLETIONS_TARGET="target/";
          # nativeBuildInputs = [ pkgs.installShellFiles ];
          # postInstall = ''
          #   installShellCompletion --bash target/felis.bash
          #   installShellCompletion --fish target/felis.fish
          #   installShellCompletion --zsh  target/_felis
          # '';
        });
      checks = nru.mkChecks (commonArgs
        // {
          inherit crate;
          # nextest = true;
        });
    };
    outputs = flake-utils.lib.eachDefaultSystem (system: let
      pkgs = import nixpkgs {
        inherit system;
        overlays = [rust-overlay.overlays.default];
      };
      nru = mkNru pkgs;

      felis = mkFelis pkgs;

      helix-plugin = mkPlugin pkgs;
    in {
      checks = felis.checks;

      packages.default = felis.crate.overrideAttrs {
        passthru = {inherit helix-plugin;};
      };

      packages.helix-plugin = helix-plugin;

      devShells.default = nru.mkDevShell {
        inputsFrom = [felis.crate];
        inherit (felis) checks;
      };
      devShells.nightly = pkgs.mkShell {
        packages = [
          pkgs.rust-bin.nightly.latest.default
        ];
      };
    });
  in
    outputs
    // {
      overlays.default = final: prev: {
        felis = outputs.packages.${final.system}.default;
      };

      overlays.withHostPkgs = final: prev: {
        felis = (mkFelis final).crate.overrideAttrs {
          passthru.helix-plugin = mkPlugin final;
        };
      };
    };
}
