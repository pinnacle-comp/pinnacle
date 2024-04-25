{
  lua52Packages,
  lua53Packages,
  lua54Packages,
  fetchurl,
  fetchzip,
  openssl,
  gnum4,
}: let
  luaHttp = with lua53Packages;
    buildLuarocksPackage {
      pname = "http";
      version = "0.4-0";
      knownRockspec =
        (fetchurl {
          url = "mirror://luarocks/http-0.4-0.rockspec";
          sha256 = "0kbf7ybjyj6408sdrmh1jb0ig5klfc8mqcwz6gv6rd6ywn47qifq";
        })
        .outPath;
      src = fetchzip {
        url = "https://github.com/daurnimator/lua-http/archive/v0.4.zip";
        sha256 = "0252mc3mns1ni98hhcgnb3pmb53lk6nzr0jgqin0ggcavyxycqb2";
      };

      #disabled = luaOlder "5.1";
      propagatedBuildInputs = [lua basexx bit32 binaryheap compat53 luaCqueues fifo lpeg lpeg_patterns luaossl];

      meta = {
        homepage = "https://github.com/daurnimator/lua-http";
        description = "HTTP library for Lua";
        license.fullName = "MIT";
      };
    };
  luaCqueues = with lua52Packages;
    buildLuarocksPackage {
      pname = "cqueues";
      version = "20200726.52-0";
      knownRockspec =
        (fetchurl {
          url = "mirror://luarocks/cqueues-20200726.52-0.rockspec";
          sha256 = "0w2kq9w0wda56k02rjmvmzccz6bc3mn70s9v7npjadh85i5zlhhp";
        })
        .outPath;
      src = fetchurl {
        url = "https://github.com/wahern/cqueues/archive/rel-20200726.tar.gz";
        sha256 = "0lhd02ag3r1sxr2hx847rdjkddm04l1vf5234v5cz9bd4kfjw4cy";
      };

      buildInputs = [
        gnum4
      ];

      externalDeps = [
        {
          name = "CRYPTO";
          dep = openssl;
        }
        {
          name = "OPENSSL";
          dep = openssl;
        }
      ];
      propagatedBuildInputs = [lua];

      meta = {
        homepage = "http://25thandclement.com/~william/projects/cqueues.html";
        description = "Continuation Queues: Embeddable asynchronous networking, threading, and notification framework for Lua on Unix.";
        license.fullName = "MIT/X11";
      };
    };
in
  with lua54Packages; # need version of http newer than in nixpkgs - check if 0.3 is ok
  
    buildLuarocksPackage {
      pname = "pinnacle-api";
      version = "dev-1";

      src = ../../api/lua;
      propagatedBuildInputs = [lua luaCqueues luaHttp lua-protobuf];
    }
