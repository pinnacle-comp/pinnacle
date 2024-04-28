{ src, craneLib, crateArgs, pinnacle-api }:
(craneLib.buildPackage (crateArgs // {
  inherit src;
  inherit (pinnacle-api) cargoArtifacts; # use pinnacle api cargo artifacts
  pname = "pinnacle-config";
  installPhaseCommand = ''
    mkdir -p $out/bin/pinnacle-config
    ls
    mv target/release/pinnacle-config $out/bin/pinnacle-config
    mv metaconfig.toml $out/bin/pinnacle-config
  '';
}))
