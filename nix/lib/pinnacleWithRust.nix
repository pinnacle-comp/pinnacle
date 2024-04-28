{ src, manifest ? null, pinnacleWithConfig, buildRustConfig }:
pinnacleWithConfig {
  pinnacle-config = buildRustConfig { inherit src; };
  inherit manifest;
}
