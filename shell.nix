{ pkgs ? import <nixpkgs> {} }:
pkgs.mkShell {
    buildInputs = [
        pkgs.gcc
        pkgs.pkg-config
        pkgs.systemd
        pkgs.seatd
        pkgs.wayland
        pkgs.libxkbcommon
        pkgs.mesa
        pkgs.libinput
        pkgs.xorg.libX11
        pkgs.xorg.libXcursor
        pkgs.xorg.libXrandr
        pkgs.xorg.libXi
        pkgs.libglvnd
        pkgs.libGL
        pkgs.libGL.dev
        pkgs.egl-wayland
        pkgs.xwayland
    ];
    shellHook = ''
        export LD_LIBRARY_PATH=$LD_LIBRARY_PATH:${pkgs.wayland}/lib:${pkgs.libxkbcommon}/lib:${pkgs.libGL}/lib
        export LUA_PATH="$LUA_PATH"
        export LUA_CPATH="$LUA_CPATH"
    '';
}
