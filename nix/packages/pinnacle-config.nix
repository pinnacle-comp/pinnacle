{
  rustPlatform,
  protobuf,
  pkg-config,
  seatd,
  libxkbcommon,
  libinput,
  lua5_4,
  libdisplay-info,
  libgbm,
}:
{src, ...}@args:
rustPlatform.buildRustPackage ((builtins.removeAttrs args ["cargoLock" "nativeBuildInputs" "buildInputs"]) // {
  PINNACLE_PROTOBUF_API_DEFS = ../../api/protobuf;
  PINNACLE_PROTOBUF_SNOWCAP_API_DEFS = ../../snowcap/api/protobuf;

  nativeBuildInputs = (args.nativeBuildInputs or []) ++ [protobuf pkg-config];
  buildInputs = (args.buildInputs or []) ++ [
    seatd.dev
    libxkbcommon
    libinput
    lua5_4
    libdisplay-info
    libgbm
  ];

  cargoLock = {
    lockFile = src + /Cargo.lock;
    allowBuiltinFetchGit = true;
  } // (args.cargoLock or {});
})
