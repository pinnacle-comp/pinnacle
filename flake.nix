{
  description = "A WIP Smithay-based Wayland compositor, inspired by AwesomeWM and configured in Lua or Rust";

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

  outputs = {
    self,
    nixpkgs,
    crane,
    fenix,
    flake-utils,
    advisory-db,
    ...
  }:
    flake-utils.lib.eachSystem ["x86_64-linux" "aarch64-linux"] (system: let
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
          (lib.hasSuffix ".rockspec" path)
          || # keep lua in build
          (lib.hasInfix "/protocol/" path)
          || # protobuf stuff
          (lib.hasInfix "/resources/" path)
          || # some resources are needed at build time
          
          # Default filter from crane (allow .rs files)
          (craneLib.filterCargoSources path type);
      };
      # Common arguments can be set here to avoid repeating them later
      commonArgs = {
        inherit src;
        strictDeps = true;

        nativeBuildInputs = [pkgs.pkg-config];
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

      individualCrateArgs =
        commonArgs
        // {
          inherit cargoArtifacts;
          inherit (craneLib.crateNameFromCargoToml {inherit src;}) version;
          # NB: we disable tests since we'll run them all via cargo-nextest
          doCheck = false;
        };

      # Build the top-level crates of the workspace as individual derivations.
      # This allows consumers to only depend on (and build) only what they need.
      # Though it is possible to build the entire workspace as a single derivation,
      # so this is left up to you on how to organize things
      pinnacle = craneLib.buildPackage (individualCrateArgs
        // {
          pname = "pinnacle";
          cargoExtraArgs = "-p pinnacle";
          inherit src;
        });
      pinnacle-api-defs = craneLib.buildPackage (individualCrateArgs
        // {
          pname = "pinnacle-api-defs";
          cargoExtraArgs = "-p pinnacle-api-defs";
          inherit src;
        });
      pinnacle-api-macros = craneLib.buildPackage (individualCrateArgs
        // {
          pname = "pinnacle-api-macros";
          cargoExtraArgs = "-p pinnacle-api-macros";
          inherit src;
        });
      pinnacle-api = craneLib.buildPackage (individualCrateArgs
        // {
          pname = "pinnacle-api";
          cargoExtraArgs = "-p pinnacle-api";
          inherit src;
        });
      # Build the pinnacle config rust
      buildPinnacleRustConfigPackage = {src}: (craneLib.buildPackage (individualCrateArgs
        // {
          inherit src;
          inherit
            (pinnacle-api)
            cargoArtifacts
            ; # use pinnacle api cargo artifacts
          pname = "pinnacle-config";
          installPhaseCommand = ''
            mkdir -p $out/bin/pinnacle-config
            ls
            mv target/release/pinnacle-config $out/bin/pinnacle-config
            mv metaconfig.toml $out/bin/pinnacle-config
          '';
        }));

      # LUA

      luaPinnacleApi = import ./nix/packages/pinnacle-api-lua.nix;

      buildPinnacleLuaConfig = {
        src ? ./api/lua/examples/test,
        extraLuaDeps ? [],
        entrypoint ? "default_config.lua",
      }: let
        name = "pinnacle-config";
        pname = "pinnacle-config";

        #luaPackages = lua.pkgs;
        luaEnv = pkgs.lua.withPackages (
          luaPackages: let
            lp = luaPackages // {pinnacle = luaPackages.callPackage ./nix/packages/pinnacle-api-lua.nix {};};
          in
            (lib.attrVals extraLuaDeps lp) ++ [(lp.callPackage luaPinnacleApi {})]
        );
      in
        pkgs.stdenv.mkDerivation {
          inherit src name pname;
          version = "";
          buildInputs = [pkgs.makeWrapper];
          installPhase = ''
            mkdir -p $out/bin
            mkdir -p $out/share/pinnacle/config
            cp * $out/share/pinnacle/config/ # placing this here for now, not sure if there's a better space
            makeWrapper ${luaEnv}/bin/lua $out/bin/${pname} --add-flags $out/share/pinnacle/config/${entrypoint}
            ln $out/bin/${pname} $out/share/pinnacle/config/${pname};
          '';
        };

      # General functions to build stuff
      # Protobuffs
      #
      protobuffs = let
        fs = pkgs.lib.fileset;
        sourceFiles = ./api/protocol;
      in
        pkgs.stdenv.mkDerivation rec {
          name = "protobuffs";
          src = fs.toSource {
            root = ./api/protocol;
            fileset = sourceFiles;
          };
          protobuffOutDir = "$out/share/config/pinnacle/protobuffs";
          postInstall = ''
            mkdir -p ${protobuffOutDir}
            cp -rv * ${protobuffOutDir}
          '';
        };
      mergePinnacleConfig = {
        # helper to join stuff together
        #symlinkJoin,
        #makeWrapper,
        pinnalcle-unwrapped ? self.packages.${system}.pinnacle,
        #writeTextFile,
        pinnacle-config ? null,
        # should be a derivation of pinnacle config - i.e. api/rust/examples/default_config.
        manifest ? null,
        # This is a derivation that contains:
        # 1. a metaconfig that has the correct runnable stuff set, such as a wrapped lua or path to the compiled rust binary
        # 2. the lua or compiled rust binary needed
        ...
      }: let
        defaultManifest = {
          #TODO: get this working with writeTextFile
          command = "./${pinnacle-config.pname}"; # run binary - will figure out lua later - maybe wrapper uses pname?
          reload_keybind = {
            modifiers = ["Ctrl" "Alt"];
            key = "r";
          };
          kill_keybind = {
            modifiers = ["Ctrl" "Alt" "Shift"];
            key = "escape";
          };
        };
        #provide a default  Toml, just in case. It may be more desireable to use

        # NOTE: as of now I can't figure out a way to check if a file exists in another derivaion.
        # I would prefer choosing a metaconfig in the order of speciifed metaconfig -> metaconfig in config directory -> default.
        # For now, the behavior is specified metaconfig -> metaconfig in directory
        manifestToml =
          pkgs.formats.toml.generate "metaconfig.toml"
          (manifest ? defaultManifest);
        manifestDerivation = pkgs.writeTextFile rec {
          name = "metaconfig.toml";
          text = builtins.readFile manifestToml;
          destination = "/bin/${name}";
        };
      in
        pkgs.symlinkJoin {
          name = "pinnacle";
          paths = [
            pinnalcle-unwrapped
            pinnacle-config
            protobuffs # protobuffs
            #manifestDerivation # currently treated as a function rather than a derivation, should be fixable.
            # For now it's copied over in pinnacle-config
          ];
          buildInputs = [pkgs.makeWrapper];
          postBuild = ''
            wrapProgram $out/bin/pinnacle \
              --add-flags "--config-dir $out/share/pinnacle/config"\
              --set PINNACLE_PROTO_DIR ${protobuffs.protobuffOutDir}\
              --prefix PATH ${pkgs.lib.makeBinPath (with pkgs; [xwayland protobuf])} # adds protobuffs to path
          '';
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
        # we can block the CI if there are issues here, but not
        # prevent downstream consumers from building our crate by itself.
        pinnacle-clippy = craneLib.cargoClippy (commonArgs
          // {
            inherit cargoArtifacts;
            cargoClippyExtraArgs = "--all-targets -- --deny warnings";
          });

        pinnacle-doc =
          craneLib.cargoDoc (commonArgs // {inherit cargoArtifacts;});

        # Check formatting
        pinnacle-fmt = craneLib.cargoFmt {inherit src;};

        # Audit dependencies

        pinnacle-audit = craneLib.cargoAudit {inherit src advisory-db;};

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

      packages = rec {
        inherit pinnacle pinnacle-api-defs pinnacle-api-macros pinnacle-api buildPinnacleLuaConfig protobuffs;
        # function to build with inputs - callable from other flakes and whatnot
        buildPinnacleRustConfig = {
          src,
          manifest ? null,
        }: (mergePinnacleConfig {
          pinnacle-config = buildPinnacleRustConfigPackage {inherit src;};
          inherit manifest;
        });
        # example of how one could use this
        exampleLuaBuild = mergePinnacleConfig {
          pinnacle-config = buildPinnacleLuaConfig {
            src = ./api/lua/examples/default;
            extraLuaDeps = ["inspect"];
          };
        };
        luaAPI = pkgs.luaPackages.callPackage luaPinnacleApi {};
      };

      apps = {pinnacle = flake-utils.lib.mkApp {drv = pinnacle;};};

      devShells.default = craneLib.devShell {
        # Inherit inputs from checks.
        checks = self.checks.${system};

        runtimeDependencies = with pkgs; [
          wayland
          mesa
          libglvnd # libEGL
        ];

        LD_LIBRARY_PATH = "${pkgs.wayland}/lib:${pkgs.libGL}/lib:${pkgs.libxkbcommon}/${pkgs.libglvnd}/lib:${pkgs.mesa.drivers}/lib";
        # Additional dev-shell environment variables can be set directly
        # MY_CUSTOM_DEVELOPMENT_VAR = "something else";

        # Extra inputs can be added here; cargo and rustc are provided by default.
        packages = [pkgs.luajitPackages.luarocks];
      };
    });
}
