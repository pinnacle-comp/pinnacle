{
  lua,
  cqueues,
  http,
  compat53,
  lua-protobuf,
  buildLuarocksPackage,
}:
# need version of http newer than in nixpkgs - check if 0.3 is ok
buildLuarocksPackage {
  pname = "pinnacle-api";
  version = "dev-1";

  src = ../../api/lua;
  propagatedBuildInputs = [lua cqueues http compat53 lua-protobuf];
}
