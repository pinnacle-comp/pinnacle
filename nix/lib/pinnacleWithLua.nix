{ pinnacleWithConfig, buildLuaConfig }:
{ src, manifest ? null, extraLuaDeps ? [ ], entrypoint }:
pinnacleWithConfig {
  inherit manifest;
  pinnacle-config = buildLuaConfig { inherit src extraLuaDeps entrypoint; };
}
