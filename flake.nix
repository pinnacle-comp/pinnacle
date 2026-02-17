{
  description = "A WIP Smithay-based Wayland compositor, inspired by AwesomeWM and configured in Lua or Rust";

  inputs = {
    # we require rustc >=1.88
    nixpkgs.url = "github:nixos/nixpkgs/nixpkgs-unstable";

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
    (flake-utils.lib.eachSystem [ "x86_64-linux" "aarch64-linux" ] (
      system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [
            (final: prev: {
              pinnacle = prev.callPackage ./nix/packages/default.nix { };
            })
          ];
        };
        fenixPkgs = fenix.packages.${system};
        toolchain = fenixPkgs.stable;
        combinedToolchain = toolchain.completeToolchain;
      in
      {
        formatter = pkgs.nixfmt;

        lib = {
          inherit (pkgs.pinnacle) buildRustConfig;
        };

        packages = {
          inherit (pkgs) pinnacle;
          default = pkgs.pinnacle;
        };

        devShell = pkgs.mkShell {
          nativeBuildInputs = [
            pkgs.pkg-config
            pkgs.lua5_4
            pkgs.libgbm
          ];
          buildInputs = with pkgs; [
            # rust devel tools
            combinedToolchain
            rust-analyzer
            cargo-outdated
            clang

            # wlcs
            (writeScriptBin "wlcs" ''
              #!/bin/sh
              ${wlcs}/libexec/wlcs/wlcs "$@"
            '')

            wayland

            # build time stuff
            protobuf
            lua54Packages.luarocks
            lua5_4

            # libs
            seatd.dev
            systemdLibs.dev
            libxkbcommon
            libinput
            mesa
            xwayland
            libdisplay-info
            libgbm
            pkg-config

            # winit on x11
            libxcursor
            libxrandr
            libxi
            libx11
          ];

          runtimeDependencies = with pkgs; [
            wayland
            mesa
            libglvnd # libEGL
            libgbm
          ];

          LD_LIBRARY_PATH = "${pkgs.wayland}/lib:${pkgs.libGL}/lib:${pkgs.libxkbcommon}/lib:${pkgs.libgbm}/lib";
        };
      }
    ))
    // {
      overlays.default = final: prev: {
        pinnacle = prev.callPackage ./nix/packages/default.nix { };
      };

      nixosModules = {
        default = import ./nix/modules/nixos;
      };

      hmModules = {
        default = import ./nix/modules/home-manager;
      };
    };
}
