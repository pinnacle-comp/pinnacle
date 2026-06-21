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
    gitignore = {
      url = "github:hercules-ci/gitignore.nix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    {
      self,
      nixpkgs,
      flake-utils,
      gitignore,
      fenix,
      ...
    }:
    let 
      vergen_env = {
        VERGEN_GIT_DIRTY = nixpkgs.lib.optionalString (self ? rev) "true";
        VERGEN_GIT_SHA = if (self ? rev) then self.rev else "<unavailable>(Nix)";
        VERGEN_GIT_BRANCH = "<unavailable>(Nix)";
        VERGEN_GIT_COMMIT_MESSAGE = "<unavailable>(Nix)";
      };
      src = nixpkgs.lib.cleanSourceWith {
        # Ignore many files that gitignoreSource doesn't ignore, see:
        # https://github.com/hercules-ci/gitignore.nix/issues/9#issuecomment-635458762
        filter =
          path: type:
          !(builtins.any (r: (builtins.match r (baseNameOf path)) != null) [
            # Nix files
            "flake.nix"
            "flake.lock"
            "nix"
            # CI files
            ".github"
            # Docs
            ".*.md"
            "wiki"
          ]);
        src = gitignore.lib.gitignoreSource ./.;
      };
    in
    (flake-utils.lib.eachSystem [ "x86_64-linux" "aarch64-linux" ] (
      system:
      let
        pkgs = import nixpkgs {
          inherit system;
          overlays = [
            (final: prev: {
              pinnacle = prev.callPackage ./nix/packages/default.nix {
                inherit vergen_env src;
              };
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
            pkgs.pinnacle.luaEnv
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
            seatd
            systemdLibs.dev
            libxkbcommon
            libinput
            mesa
            xwayland
            libdisplay-info
            libgbm

            # winit on x11
            libxcursor
            libxrandr
            libxi
            libx11
            alacritty
          ];

          runtimeDependencies = with pkgs; [
            wayland
            mesa
            libglvnd # libEGL
            libgbm
          ];
          NIX_LD_LIBRARY_PATH = "${pkgs.lib.makeLibraryPath (
            with pkgs;
            [
              wayland
              lua5_4
              libinput
              libxkbcommon
              libdisplay-info
              seatd
              libgbm
              udev
            ]
          )}";
          shellHook = ''
            export LD_LIBRARY_PATH="$LD_LIBRARY_PATH:$NIX_LD_LIBRARY_PATH"
          '';
        };
      }
    ))
    // {
      overlays.default = final: prev: {
        pinnacle = prev.callPackage ./nix/packages/default.nix {
          inherit vergen_env src;
        };
      };

      nixosModules = {
        default = import ./nix/modules/nixos;
      };

      hmModules = {
        default = import ./nix/modules/home-manager;
      };
    };
}
