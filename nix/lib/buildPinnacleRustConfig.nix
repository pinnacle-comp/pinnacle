{ craneLib, crateArgs, pinnacle-api }:
{ src }:
(craneLib.buildPackage (crateArgs // rec {
  inherit src;
  inherit (pinnacle-api) cargoArtifacts; # use pinnacle api cargo artifacts
  pname = "pinnacle-config";
  installPhaseCommand = ''
    mkdir -p $out/share/pinnacle/config/
    mkdir $out/bin
    mv target/release/${pname} $out/bin/
    mv metaconfig.toml $out/share/pinnacle/config/
    ln $out/bin/${pname} $out/share/pinnacle/config/${pname};
  '';
}))
