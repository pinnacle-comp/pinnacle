{
  rustPlatform,
  lib,
  pkg-config,
  wayland,
  lua54Packages,
  lua5_4,
  extraLuaPackages ? (ps: [ ]),
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
  libxcursor,
  libxi,
  libxrandr,
  libx11,
}:
let
  buildRustConfig = callPackage ./pinnacle-config.nix { };

  meta = {
    description = "A WIP Smithay-based Wayland compositor, inspired by AwesomeWM and configured in Lua or Rust";
    homepage = "https://pinnacle-comp.github.io/pinnacle/";
    license = lib.licenses.gpl3;
    maintainers = [ "pinnacle-comp" ];
  };
  version = "0.2.3";

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
    knownRockspec = ../../api/lua/rockspecs/pinnacle-api-0.2.2-1.rockspec;
    propagatedBuildInputs = with lua54Packages; [
      cqueues
      http
      lua-protobuf
      compat53
      luaposix
    ];

    postInstall = ''
      mkdir -p $out/share/pinnacle/protobuf/pinnacle
      cp -rL --no-preserve ownership,mode ../../api/protobuf/pinnacle $out/share/pinnacle/protobuf
      mkdir -p $out/share/pinnacle/snowcap/protobuf/snowcap
      cp -rL --no-preserve ownership,mode ../../snowcap/api/protobuf/snowcap $out/share/pinnacle/snowcap/protobuf
      mkdir -p $out/share/pinnacle/protobuf/google
      cp -rL --no-preserve ownership,mode ../../api/protobuf/google $out/share/pinnacle/protobuf
      mkdir -p $out/share/pinnacle/snowcap/protobuf/google
      cp -rL --no-preserve ownership,mode ../../snowcap/api/protobuf/google $out/share/pinnacle/snowcap/protobuf
      find $out/share/pinnacle
    '';
  };
in
rustPlatform.buildRustPackage (finalAttrs: {
  inherit meta version;

  pname = "pinnacle-server";
  src = ../..;
  cargoLock = {
    lockFile = "${../..}/Cargo.lock";
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
    libxcursor
    libxrandr
    libxi
    libx11
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

  checkFeatures = [ "testing" ];
  checkNoDefaultFeatures = true;
  cargoTestFlags = [
    "--exclude"
    "wlcs_pinnacle"
    "--all"
    "--"
    "--skip"
    "process_spawn"
  ];

  preCheck = ''
    export LD_LIBRARY_PATH="${lib.makeLibraryPath [ wayland ]}";
    export XDG_RUNTIME_DIR=$(mktemp -d)
  '';

  postInstall = ''
    wrapProgram $out/bin/pinnacle --prefix PATH ":" ${
      lib.makeBinPath [
        rustc
        cargo
        finalAttrs.passthru.luaEnv
        xwayland
      ]
    }
    install -m755 ./resources/pinnacle-session $out/bin/pinnacle-session
    mkdir -p $out/share/wayland-sessions
    install -m644 ./resources/pinnacle.desktop $out/share/wayland-sessions/pinnacle.desktop
    patchShebangs $out/bin/pinnacle-session
    mkdir -p $out/share/xdg-desktop-portal
    install -m644 ./resources/pinnacle-portals.conf $out/share/xdg-desktop-portal/pinnacle-portals.conf
    install -m644 ./resources/pinnacle-portals.conf $out/share/xdg-desktop-portal/pinnacle-uwsm-portals.conf
  '';

  runtimeDependencies = [
    wayland
    mesa
    libglvnd # libEGL
  ];

  passthru = {
    luaEnv = lua5_4.withPackages (
      ps:
      [
        finalAttrs.passthru.lua-client-api
        ps.cjson
      ]
      ++ (extraLuaPackages ps)
    );
    inherit buildRustConfig;
    providedSessions = [ "pinnacle" ];
    lua-client-api = lua-client-api;
  };
})
