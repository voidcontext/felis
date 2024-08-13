{
  inputs.nixpkgs.url = "nixpkgs/release-24.05";
  inputs.flake-utils.url = "github:numtide/flake-utils";

  inputs.crane = {
    url = "github:ipetkov/crane/v0.18.0";
    inputs.nixpkgs.follows = "nixpkgs";
  };

  outputs = {
    self,
    nixpkgs,
    flake-utils,
    crane,
    ...
  }: let
    mkFelis = import ./nix/felis.nix;

    outputs = flake-utils.lib.eachDefaultSystem (
      system: let
        pkgs = import nixpkgs {inherit system;};
        craneLib = crane.mkLib pkgs;
        callPackage = pkgs.lib.callPackageWith (pkgs // {inherit craneLib;});
        felis = callPackage mkFelis {};
      in {
        checks = felis.checks;

        packages.default = felis;
        packages.helix-plugin = felis.helix-plugin;

        devShells.default = craneLib.devShell {
          checks = self.checks.${system};
          packages = [
            pkgs.rust-analyzer
            pkgs.cargo-outdated
          ];
        };
      }
    );
  in
    outputs
    // {
      overlays.default = final: prev: {
        felis = outputs.packages.${final.system}.default;
      };

      overlays.withHostPkgs = final: prev: let
        callPackage = final.lib.callPackageWith (final // {inherit crane;});
      in {
        felis = callPackage mkFelis {};
      };
    };
}
