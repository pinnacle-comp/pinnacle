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
  writeScriptBin,
  wlcs,
  rustc,
  cargo,
  makeWrapper,
  callPackage,
  libglvnd,
  autoPatchelfHook,
}:
let
  pinnacle = ../..;
  wlcs-script = writeScriptBin "wlcs" ''
    #!/bin/sh
    ${wlcs}/libexec/wlcs/wlcs "$@"
  '';
  buildRustConfig = callPackage ./pinnacle-config.nix { };

  meta = {
    description = "A WIP Smithay-based Wayland compositor, inspired by AwesomeWM and configured in Lua or Rust";
    homepage = "https://pinnacle-comp.github.io/pinnacle/";
    license = lib.licenses.gpl3;
    maintainers = [ "pinnacle-comp" ];
  };
  version = "0.1.0";

  luaClient = lua54Packages.buildLuarocksPackage {
    inherit meta version;
    pname = "pinnacle";
    src = ../../api/lua;
    propagatedBuildInputs = [lua5_4];
  };
  lua = lua5_4.withPackages (ps: [ ps.luarocks luaClient]);
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

  buildFeatures = [ "wlcs" ];

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
    wlcs

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
    git
    wayland
    wlcs-script
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
        wlcs-script
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
    inherit buildRustConfig;
    providedSessions = [ "pinnacle" ];
  };
}
