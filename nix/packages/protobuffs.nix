{ lib, stdenv }:
let
  fs = lib.fileset;
  sourceFiles = ../../pinnacle-api-defs/protocol;
in stdenv.mkDerivation rec {
  name = "protobuffs";
  src = fs.toSource {
    root = ../../pinnacle-api-defs/protocol;
    fileset = sourceFiles;
  };
  protobuffOutDir = "$out/share/config/pinnacle/protobuffs";
  postInstall = ''
    mkdir -p ${protobuffOutDir}
    cp -rv * ${protobuffOutDir}
  '';
}

