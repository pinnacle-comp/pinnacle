{
  description = "Build a cargo workspace";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";

    crane = {
      url = "github:ipetkov/crane";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
      inputs.rust-analyzer-src.follows = "";
    };

    flake-utils.url = "github:numtide/flake-utils";

    advisory-db = {
      url = "github:rustsec/advisory-db";
      flake = false;
    };
  };

  outputs = { self, nixpkgs, crane, fenix, flake-utils, advisory-db, ... }:
    flake-utils.lib.eachSystem [ "x86_64-linux" "aarch64-linux" ] (system:
      let
        pkgs = nixpkgs.legacyPackages.${system};
        fenixPkgs = fenix.packages.${system};
        toolchain = fenixPkgs.stable;
        combinedToolchain = toolchain.completeToolchain;
        inherit (pkgs) lib;

        craneLib = (crane.mkLib pkgs).overrideToolchain combinedToolchain;

        # Get the relevant files for the rust build
        src = lib.cleanSourceWith {
          src = ./.; # The original, unfiltered source
          filter = path: type:
            (lib.hasSuffix ".rockspec" path) || # keep lua in build
            (lib.hasInfix "/protocol/" path) || # protobuf stuff
            (lib.hasInfix "/resources/" path)
            || # some resources are needed at build time

            # Default filter from crane (allow .rs files)
            (craneLib.filterCargoSources path type);
        };
        # Common arguments can be set here to avoid repeating them later
        commonArgs = {
          inherit src;
          strictDeps = true;

          nativeBuildInputs = [ pkgs.pkg-config ];
          buildInputs = with pkgs; [
            wayland

            # build time stuff
            protobuf
            lua54Packages.luarocks

            # libs
            seatd.dev
            systemdLibs.dev
            libxkbcommon
            libinput
            mesa
            xwayland

            # winit on x11
            xorg.libXcursor
            xorg.libXrandr
            xorg.libXi
            xorg.libX11
          ];

          # Enironment Variables to set as necessary
          PROTOC = "${pkgs.protobuf}/bin/protoc";
        };

        # Build *just* the cargo dependencies (of the entire workspace),
        # so we can reuse all of that work (e.g. via cachix) when running in CI
        # It is *highly* recommended to use something like cargo-hakari to avoid
        # cache misses when building individual top-level-crates
        cargoArtifacts = craneLib.buildDepsOnly commonArgs;

        individualCrateArgs = commonArgs // {
          inherit cargoArtifacts;
          inherit (craneLib.crateNameFromCargoToml { inherit src; }) version;
          # NB: we disable tests since we'll run them all via cargo-nextest
          doCheck = false;
        };

        # Build the top-level crates of the workspace as individual derivations.
        # This allows consumers to only depend on (and build) only what they need.
        # Though it is possible to build the entire workspace as a single derivation,
        # so this is left up to you on how to organize things
        pinnacle = craneLib.buildPackage (individualCrateArgs // {
          pname = "pinnacle";
          cargoExtraArgs = "-p pinnacle";
          inherit src;
        });
        pinnacle-api-defs = craneLib.buildPackage (individualCrateArgs // {
          pname = "pinnacle-api-defs";
          cargoExtraArgs = "-p pinnacle-api-defs";
          inherit src;
        });
        pinnacle-api-macros = craneLib.buildPackage (individualCrateArgs // {
          pname = "pinnacle-api-macros";
          cargoExtraArgs = "-p pinnacle-api-macros";
          inherit src;
        });
        pinnacle-api = craneLib.buildPackage (individualCrateArgs // {
          pname = "pinnacle-api";
          cargoExtraArgs = "-p pinnacle-api";
          inherit src;
        });

        protobuffs = pkgs.callPackage ./nix/packages/protobuffs.nix { };
        luaPinnacleApi = import ./nix/packages/pinnacle-api-lua.nix;

        pinnacleLib = import ./nix/lib {
          inherit (pkgs) lib newScope;
          inherit craneLib pinnacle-api luaPinnacleApi pinnacle protobuffs;
          crateArgs = individualCrateArgs;
        };

      in {
        formatter = pkgs.nixfmt;
        checks = {
          # Build the crates as part of `nix flake check` for convenience
          inherit pinnacle pinnacle-api-defs pinnacle-api-macros pinnacle-api;

          # Run clippy (and deny all warnings) on the workspace source,
          # again, reusing the dependency artifacts from above.
          #
          # Note that this is done as a separate derivation so that
          # we can block the  {}CI if there are issues here, but not
          # prevent downstream consumers from building our crate by itself.
          pinnacle-clippy = craneLib.cargoClippy (commonArgs // {
            inherit cargoArtifacts;
            cargoClippyExtraArgs = "--all-targets -- --deny warnings";
          });

          pinnacle-doc =
            craneLib.cargoDoc (commonArgs // { inherit cargoArtifacts; });

          # Check formatting
          pinnacle-fmt = craneLib.cargoFmt { inherit src; };

          # Audit dependencies

          pinnacle-audit = craneLib.cargoAudit { inherit src advisory-db; };

          # Run tests with cargo-nextest
          #
          # test currently modify state, so I've disabled them in the check
          #
          # pinnacle-nextest = craneLib.cargoNextest (commonArgs // {
          #   inherit cargoArtifacts;
          #   partitions = 1;
          #   partitionType = "count";
          # });
        };
        lib = pinnacleLib;
        packages = {
          inherit pinnacle protobuffs;
          inherit (pinnacleLib) pinnacleWithRust pinnacleWithLua;
        };

        apps = { pinnacle = flake-utils.lib.mkApp { drv = pinnacle; }; };

        devShells.default = craneLib.devShell {
          # Inherit inputs from checks.
          checks = self.checks.${system};

          runtimeDependencies = with pkgs; [
            wayland
            mesa
            libglvnd # libEGL
          ];

          LD_LIBRARY_PATH =
            "${pkgs.wayland}/lib:${pkgs.libGL}/lib:${pkgs.libxkbcommon}/${pkgs.libglvnd}/lib:${pkgs.mesa.drivers}/lib";
          # Additional dev-shell environment variables can be set directly
          # MY_CUSTOM_DEVELOPMENT_VAR = "something else";

          # Extra inputs can be added here; cargo and rustc are provided by default.
          packages = [ pkgs.luajitPackages.luarocks ];
        };
      });
}
