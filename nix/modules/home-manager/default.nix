{
  pkgs,
  config,
  lib,
  ...
}:
let
  cfg = config.wayland.windowManager.pinnacle;
  settingsFormat = pkgs.formats.toml { };
  systemdModule = {
    options = with lib.options; {
      enable = mkOption {
        default = true;
        example = true;
        type = lib.types.bool;
        description = ''
          create and enable the systemd user service to manage pinnacle. not enabling this option means you will need to create the user service/shutdown target yourself.
        '';
      };
      useService = mkOption {
        default = true;
        example = true;
        type = lib.types.bool;
        description = "use a systemd service rather than a target -- needed for the provided pinnacle-session command but not necessary if using UWSM to manage the pinnacle session.";
      };
      xdgAutostart = mkEnableOption "autostart xdg applications";
    };
  };
in
with lib.options;
{
  options.wayland.windowManager.pinnacle = {
    enable = mkEnableOption "pinnacle";

    package = mkPackageOption pkgs "pinnacle" {
      default = "pinnacle";
      example = "pkgs.pinnacle";
      extraDescription = "package containing the pinnacle server binary";
    };

    clientPackage = mkPackageOption pkgs "pinnacle-config" {
      default = "pinnacle-config";
      example = "pkgs.pinnacle-config";
      extraDescription = "package containing the command/script to run as the pinnacle user configuration.";
    };

    config = {
      execCmd = mkOption {
        type = lib.types.listOf (
          lib.types.oneOf (
            with lib.types;
            [
              str
              path
            ]
          )
        );
        default = [ "${cfg.clientPackage}/bin/pinnacle-config" ];
        example = ''["''${pkgs.pinnacle-config}/bin/pinnacle-config"]'';
        description = ''
          the command to run for the pinnacle user configuration, provided via the pinnacle config toml file to the pinnacle server binary.
          this defaults to ''${pkgs.pinnacle-config}/bin/pinnacle-config -- you can provide this package via a nixpkgs overlay like:

          ```nix
            pkgs = import nixpkgs {
              inherit system;
              overlays = [
                inputs.pinnacle.overlays.default
                (final: prev: {
                  pinnacle-config = prev.pinnacle.buildRustConfig {
                    pname = "pinnacle-config";
                    version = "0.1.0";
                    src = ./.;
                  };
                })
              ];
            };
          ```

          or by setting the package option directly.

          please note that if you're running this home-manager module on a non-NixOS distribution and making use of snowcap, you need to wrap
          the call to your configuration script/executable in `nixGL` to ensure the fallback to software rendering isn't used --
          see: https://pinnacle-comp.github.io/pinnacle/getting-started/running#from-source. you should not use `nix run` here, however. instead,
          make sure the `nixGL` and `nixVulkanIntel` packages are available and invoke each:

          ```nix
            services.wayland.windowManager.pinnacle.config.execCmd = ["''${pkgs.nixGL}/bin/nixGL" "''${pkgs.nixVulkanIntel}/bin/nixVulkanIntel" "''${pkgs.pinnacle-config}/bin/pinnacle-config"];
          ```
        '';
      };
    };

    systemd = lib.mkOption {
      type = lib.types.submodule systemdModule;
    };

    extraSettings = mkOption {
      type = lib.types.attrs;

      default = { };

      example = ''
        ```nix
          programs.pinnacle.extraSettings = {
            env = {
              "MY_ENV_VAR" = "super special env var";
            };
        };
        ```
      '';

      description = ''
        the pinnacle.toml configuration settings exposed as a nix attrset -- these are merged with the settings exposed under the `config` attr.

        see: https://pinnacle-comp.github.io/pinnacle/
      '';
    };

    mergedSettings = mkOption {
      internal = true;
      type = settingsFormat.type;
      default = {
        run = cfg.config.execCmd;
      }
      // cfg.extraSettings;
    };
  };

  config =
    let
      configFile = settingsFormat.generate "pinnacle.toml" cfg.mergedSettings;
    in
    lib.mkIf cfg.enable {
      home.packages = [
        cfg.package
        cfg.clientPackage
        pkgs.protobuf
        pkgs.xwayland
      ];

      xdg.configFile."pinnacle/pinnacle.toml" = {
        source = configFile;
        onChange = ''
          PATH="${pkgs.protobuf}/bin:''${PATH}" ${cfg.package}/bin/pinnacle client -e "Pinnacle.reload_config()"
        '';
      };

      xdg.dataFile = {
        "pinnacle" = {
          source = "${cfg.package.lua-client-api}/share/pinnacle";
          force = true;
          onChange = ''
            PATH="${pkgs.protobuf}/bin:''${PATH}" ${cfg.package}/bin/pinnacle client -e "Pinnacle.reload_config()"
          '';
        };
      };

      systemd.user.services.pinnacle = lib.mkIf (cfg.systemd.enable && cfg.systemd.useService) {
        Unit = {
          Description = "A Wayland compositor inspired by AwesomeWM";
          BindsTo = [ "graphical-session.target" ];
          Wants = [
            "graphical-session-pre.target"
          ]
          ++ lib.optionals cfg.systemd.xdgAutostart [ "xdg-desktop-autostart.target" ];
          After = [ "graphical-session-pre.target" ];
          Before = [
            "graphical-session.target"
          ]
          ++ lib.optionals cfg.systemd.xdgAutostart [ "xdg-desktop-autostart.target" ];
        };
        Service = {
          Slice = [ "session.slice" ];
          Type = "notify";
          ExecStart = "${cfg.package}/bin/pinnacle --session";
        };
      };

      systemd.user.targets.pinnacle-shutdown = lib.mkIf (cfg.systemd.enable && cfg.systemd.useService) {
        Unit = {
          Description = "Shutdown running Pinnacle session";
          DefaultDependencies = false;
          StopWhenUnneeded = true;

          Conflicts = [
            "graphical-session.target"
            "graphical-session-pre.target"
          ];
          After = [
            "graphical-session.target"
            "graphical-session-pre.target"
          ];
        };
      };

      systemd.user.targets.pinnacle-session = lib.mkIf (cfg.systemd.enable && !cfg.systemd.useService) {
        Unit = {
          Description = "Pinnacle compositor session";
          Documentation = [ "man:systemd.special(7)" ];
          BindsTo = [ "graphical-session.target" ];
          Wants = [
            "graphical-session-pre.target"
          ]
          ++ lib.optionals cfg.systemd.xdgAutostart [ "xdg-desktop-autostart.target" ];
          After = [ "graphical-session-pre.target" ];
          Before = lib.optionals cfg.systemd.xdgAutostart [ "xdg-desktop-autostart.target" ];
        };
      };
    };
}
