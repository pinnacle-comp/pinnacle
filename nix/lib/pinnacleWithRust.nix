{ pinnacleWithConfig, buildRustConfig }:
{ src, manifest ? null }:
pinnacleWithConfig {
  inherit manifest;
  pinnacle-config = buildRustConfig { inherit src; };
}
