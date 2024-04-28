{ lib, newScope, craneLib, crateArgs, pinnacle-api }:
lib.makeScope newScope (self:
  let inherit (self) callPackage;

  in {
    buildLuaConfig = callPackage ./buildPinnacleLuaConfig.nix { };
    buildRustConfig = callPackage ./buildPinnacleRustConfig.nix {
      inherit craneLib crateArgs pinnacle-api;
    };
    pinnacleWithConfig = callPackage ./merge-pinnacle-config.nix { };
    pinnacleWithRust = callPackage ./pinnacleWithRust.nix { };
    pinnacleWithLua = { };
  })
