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

        # things in the filter are allowed in the nix build
        src = lib.cleanSourceWith {
          src = ./.; # The original, unfiltered source
          filter = path: type:
            (lib.hasSuffix ".lua" path) || (lib.hasSuffix ".rpckspec" path)
            || # keep lua in build
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
      in {
        formatter = pkgs.nixfmt;
        checks = {
          # Build the crates as part of `nix flake check` for convenience
          inherit pinnacle pinnacle-api-defs pinnacle-api-macros pinnacle-api;

          # Run clippy (and deny all warnings) on the workspace source,
          # again, reusing the dependency artifacts from above.
          #
          # Note that this is done as a separate derivation so that
          # we can block the CI if there are issues here, but not
          # prevent downstream consumers from building our crate by itself.
          my-workspace-clippy = craneLib.cargoClippy (commonArgs // {
            inherit cargoArtifacts;
            cargoClippyExtraArgs = "--all-targets -- --deny warnings";
          });

          my-workspace-doc =
            craneLib.cargoDoc (commonArgs // { inherit cargoArtifacts; });

          # Check formatting
          my-workspace-fmt = craneLib.cargoFmt { inherit src; };

          # Audit dependencies
          my-workspace-audit = craneLib.cargoAudit { inherit src advisory-db; };

          # Audit licenses
          my-workspace-deny = craneLib.cargoDeny { inherit src; };

          # Run tests with cargo-nextest
          # Consider setting `doCheck = false` on other crate derivations
          # if you do not want the tests to run twice
          my-workspace-nextest = craneLib.cargoNextest (commonArgs // {
            inherit cargoArtifacts;
            partitions = 1;
            partitionType = "count";
          });

          # Ensure that cargo-hakari is up to date
          my-workspace-hakari = craneLib.mkCargoDerivation {
            inherit src;
            pname = "my-workspace-hakari";
            cargoArtifacts = null;
            doInstallCargoArtifacts = false;

            buildPhaseCargoCommand = ''
              cargo hakari generate --diff  # workspace-hack Cargo.toml is up-to-date
              cargo hakari manage-deps --dry-run  # all workspace crates depend on workspace-hack
              cargo hakari verify
            '';

            nativeBuildInputs = [ pkgs.cargo-hakari ];
          };
        };

        packages = {
          inherit pinnacle pinnacle-api-defs pinnacle-api-macros pinnacle-api;
        };

        apps = { pinnacle = flake-utils.lib.mkApp { drv = pinnacle; }; };

        devShells.default = craneLib.devShell {
          # Inherit inputs from checks.
          checks = self.checks.${system};

          # Additional dev-shell environment variables can be set directly
          # MY_CUSTOM_DEVELOPMENT_VAR = "something else";

          # Extra inputs can be added here; cargo and rustc are provided by default.
          packages = [ pkgs.cargo-hakari ];
        };
      });
}
