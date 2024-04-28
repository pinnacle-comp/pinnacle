{ src ? ./api/lua/examples/test, extraLuaDeps ? [ ], entrypoint ? "init.lua"
, lua, luaPinnacleApi, lib, stdenv, makeWrapper }:
let
  name = "pinnacle-config";
  pname = "pinnacle-config";

  #luaPackages = lua.pkgs;
  luaEnv = lua.withPackages (luaPackages:
    let
      lp = luaPackages // {
        pinnacle =
          luaPackages.callPackage ./nix/packages/pinnacle-api-lua.nix { };
      };
    in (lib.attrVals extraLuaDeps lp)
    ++ [ (lp.callPackage luaPinnacleApi { }) ]);
in stdenv.mkDerivation {
  inherit src name pname;
  version = "";
  buildInputs = [ makeWrapper ];
  installPhase = ''
    mkdir -p $out/bin
    mkdir -p $out/share/pinnacle/config
    cp * $out/share/pinnacle/config/ # placing this here for now, not sure if there's a better space
    makeWrapper ${luaEnv}/bin/lua $out/bin/${pname} --add-flags $out/share/pinnacle/config/${entrypoint}
    ln $out/bin/${pname} $out/share/pinnacle/config/${pname};
  '';
}
