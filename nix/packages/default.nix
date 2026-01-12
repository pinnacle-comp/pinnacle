{
  rustPlatform,
  lib,
  pkg-config,
  xorg,
  wayland,
  lua54Packages,
  lua5_4,
  protobuf,
  seatd,
  systemdLibs,
  libxkbcommon,
  mesa,
  xwayland,
  libinput,
  libdisplay-info,
  git,
  libgbm,
  rustc,
  cargo,
  makeWrapper,
  callPackage,
  libglvnd,
  autoPatchelfHook,
  fetchzip,
  fetchurl,
  libxcrypt,
}:
let
  pinnacle = ../..;
  buildRustConfig = callPackage ./pinnacle-config.nix { };

  meta = {
    description = "A WIP Smithay-based Wayland compositor, inspired by AwesomeWM and configured in Lua or Rust";
    homepage = "https://pinnacle-comp.github.io/pinnacle/";
    license = lib.licenses.gpl3;
    maintainers = [ "pinnacle-comp" ];
  };
  version = "0.2.1";

  # we need a newer version of luaposix than what's in nixpkgs
  luaposix = lua54Packages.luaposix.overrideAttrs (old: rec {
    version = "36.3.0-1";
    knownRockspec =
      (fetchurl {
        url = "mirror://luarocks/luaposix-36.3-1.rockspec";
        sha256 = "sha256-6/sAsOWrrXjdzPlAp/Z5FetQfzrkrf6TmOz3FZaBiks=";
      }).outPath;
    src = fetchzip {
      url = "http://github.com/luaposix/luaposix/archive/v36.3.zip";
      sha256 = "sha256-RKDH1sB7r7xDqueByWwps5fBfl5GBL9L86FjzfStBUw=";
    };

    disabled = lua54Packages.luaOlder "5.1" || lua54Packages.luaAtLeast "5.5";
    propagatedBuildInputs = with lua54Packages; [
      bit32
      std-normalize
      libxcrypt
      pkg-config
    ];
    meta.broken = disabled;
  });

  lua-client-api = lua54Packages.buildLuarocksPackage rec {
    inherit meta version;
    pname = "pinnacle-client-api";
    src = lib.fileset.toSource {
      root = ../..;
      # we should probably filter out parts of the repo that aren't relevant but this at least works
      fileset = lib.fileset.unions [
        ../../api
        ../../snowcap
      ];
    };
    sourceRoot = "${src.name}/api/lua";
    knownRockspec = ../../api/lua/rockspecs/pinnacle-api-0.2.0-1.rockspec;
    propagatedBuildInputs = with lua54Packages; [
      cqueues
      http
      lua-protobuf
      compat53
      luaposix
    ];

    postInstall = ''
      mkdir -p $out/share/pinnacle/protobuf/pinnacle
      cp -rL --no-preserve ownership,mode ${../..}/api/protobuf/pinnacle $out/share/pinnacle/protobuf
      mkdir -p $out/share/pinnacle/snowcap/protobuf/snowcap
      cp -rL --no-preserve ownership,mode ${../..}/snowcap/api/protobuf/snowcap $out/share/pinnacle/snowcap/protobuf
      mkdir -p $out/share/pinnacle/protobuf/google
      cp -rL --no-preserve ownership,mode ${../..}/api/protobuf/google $out/share/pinnacle/protobuf
      mkdir -p $out/share/pinnacle/snowcap/protobuf/google
      cp -rL --no-preserve ownership,mode ${../..}/snowcap/api/protobuf/google $out/share/pinnacle/snowcap/protobuf
      find $out/share/pinnacle
    '';
  };
  buildLuaConfig = args: callPackage ./pinnacle-lua-config (args // { inherit lua-client-api; });
  lua = lua5_4.withPackages (ps: [ lua-client-api ps.cjson ]);
in
rustPlatform.buildRustPackage {
  inherit meta version;

  pname = "pinnacle-server";
  src = pinnacle;
  cargoLock = {
    lockFile = "${pinnacle}/Cargo.lock";
    # as we're not in-tree in nixpkgs right now, we don't benefit from the public nix subsituters.
    # consequently, we can neither provide a single static `cargoHash` nor a set of hashes for just
    # the dependencies fetched via git (these can change since cargo doesn't pin the git revision).
    # so we're stuck doing this until we can upstream the package.
    allowBuiltinFetchGit = true;
  };

  buildInputs = [
    wayland

    # libs
    seatd.dev
    systemdLibs.dev
    libxkbcommon
    libinput
    mesa
    xwayland
    libdisplay-info
    libgbm
    lua5_4

    # winit on x11
    xorg.libXcursor
    xorg.libXrandr
    xorg.libXi
    xorg.libX11
  ];

  nativeBuildInputs = [
    pkg-config
    protobuf
    lua54Packages.luarocks
    lua5_4
    lua-client-api
    git
    wayland
    makeWrapper
    autoPatchelfHook
  ];

  # integration tests don't work inside the nix sandbox, I think because the wayland socket is inaccessible.
  cargoTestFlags = [ "--lib" ];
  # the below is necessary to actually execute the integration tests
  # TODO:
  #   1. figure out if it's possible to run the integration tests inside the nix sandbox
  #   2. fix the RPATH of the test binary prior to execution so LD_LIBRARY_PATH isn't necessary (it should be avoided with nix)
  # preCheck = ''
  #   export LD_LIBRARY_PATH="${wayland}/lib:${libGL}/lib:${libxkbcommon}/lib"
  # '';

  postInstall = ''
    wrapProgram $out/bin/pinnacle --prefix PATH ":" ${
      lib.makeBinPath [
        rustc
        cargo
        lua
        xwayland
      ]
    }
    install -m755 ${../../resources/pinnacle-session} $out/bin/pinnacle-session
    mkdir -p $out/share/wayland-sessions
    install -m644 ${../../resources/pinnacle.desktop} $out/share/wayland-sessions/pinnacle.desktop
    patchShebangs $out/bin/pinnacle-session
    mkdir -p $out/share/xdg-desktop-portal
    install -m644 ${../../resources/pinnacle-portals.conf} $out/share/xdg-desktop-portal/pinnacle-portals.conf
    install -m644 ${../../resources/pinnacle-portals.conf} $out/share/xdg-desktop-portal/pinnacle-uwsm-portals.conf
  '';

  runtimeDependencies = [
    wayland
    mesa
    libglvnd # libEGL
  ];

  passthru = {
    inherit buildRustConfig buildLuaConfig;
    providedSessions = [ "pinnacle" ];
    lua-client-api = lua-client-api;
  };
}
