{
  description = " A WIP Smithay-based Wayland compositor, inspired by AwesomeWM and configured in Lua or Rust";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-23.11";

    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs =
    {
      nixpkgs,
      flake-utils,
      fenix,
      ...
    }:
    flake-utils.lib.eachSystem
      [
        "x86_64-linux"
        "aarch64-linux"
      ]
      (
        system:
        let
          pkgs = nixpkgs.legacyPackages.${system};
          fenixPkgs = fenix.packages.${system};
          toolchain = fenixPkgs.stable;
          combinedToolchain = toolchain.completeToolchain;
        in
        {
          devShell = pkgs.mkShell {
            buildInputs = [
              # rust devel tools
              combinedToolchain
              pkgs.rust-analyzer
              pkgs.cargo-outdated

              # build time stuff
              pkgs.pkg-config
              pkgs.protobuf
              pkgs.luarocks

              # libs
              pkgs.seatd.dev
              pkgs.systemdLibs.dev
              pkgs.libxkbcommon
              pkgs.libinput
              pkgs.mesa
            ];
            LD_LIBRARY_PATH = "${pkgs.wayland}/lib:${pkgs.libGL}/lib";
          };
        }
      );
}
