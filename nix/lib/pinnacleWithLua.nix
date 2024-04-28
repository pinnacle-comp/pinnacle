{ src, manifest ? null, pinnacleWithConfig, buildLuaConfig, extraLuaDeps ? [ ]
, entryp }:
pinnacleWithConfig {
  pinnacle-config = buildLuaConfig { inherit src extraLuaDeps; };
}
