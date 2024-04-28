{ lib, newScope, craneLib, crateArgs, pinnacle-api, luaPinnacleApi, pinnacle
, protobuffs }:
lib.makeScope newScope (self:
  let inherit (self) callPackage;

  in {
    buildLuaConfig =
      callPackage ./buildPinnacleLuaConfig.nix { inherit luaPinnacleApi; };
    buildRustConfig = callPackage ./buildPinnacleRustConfig.nix {
      inherit craneLib crateArgs pinnacle-api;
    };
    pinnacleWithConfig = callPackage ./merge-pinnacle-config.nix {
      pinnacle-unwrapped = pinnacle;
      inherit protobuffs;
    };
    pinnacleWithRust = callPackage ./pinnacleWithRust.nix { };
    pinnacleWithLua = callPackage ./pinnacleWithLua.nix { };
  })
