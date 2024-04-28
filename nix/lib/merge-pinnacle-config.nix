{
# helper to join stuff together
#symlinkJoin,
#makeWrapper,
pinnacle-unwrapped,
#writeTextFile,
protobuffs,
# should be a derivation of pinnacle config - i.e. api/rust/examples/default_config. 
lib, symlinkJoin, formats, writeTextFile, makeWrapper, xwayland, protobuf,

# This is a derivation that contains:
# 1. a metaconfig that has the correct runnable stuff set, such as a wrapped lua or path to the compiled rust binary
# 2. the lua or compiled rust binary needed
... }:
{ manifest ? { }, pinnacle-config ? null }:
let
  defaultManifest = {
    #TODO: get this working with writeTextFile
    command =
      "./${pinnacle-config.pname}"; # run binary - will figure out lua later - maybe wrapper uses pname?
    reload_keybind = {
      modifiers = [ "Ctrl" "Alt" ];
      key = "r";
    };
    kill_keybind = {
      modifiers = [ "Ctrl" "Alt" "Shift" ];
      key = "escape";
    };
  };
  #provide a default  Toml, just in case. It may be more desireable to use

  # NOTE: as of now I can't figure out a way to check if a file exists in another derivaion.
  # I would prefer choosing a metaconfig in the order of speciifed metaconfig -> metaconfig in config directory -> default.
  # For now, the behavior is specified metaconfig -> metaconfig in directory
  manifestToml =
    formats.toml.generate "metaconfig.toml" (defaultManifest // manifest);
  manifestDerivation = writeTextFile rec {
    name = "metaconfig.toml";
    text = builtins.readFile manifestToml;
    destination = "/bin/${name}";
  };
in symlinkJoin {
  name = "pinnacle";
  paths = [
    pinnacle-unwrapped
    pinnacle-config
    protobuffs # protobuffs
    #(manifestDerivation) # currently treated as a function rather than a derivation, should be fixable.
    # For now it's copied over in pinnacle-config
  ];
  buildInputs = [ makeWrapper ];
  postBuild = ''
    wrapProgram $out/bin/pinnacle \
      --add-flags "--config-dir $out/share/pinnacle/config"\
      --set PINNACLE_PROTO_DIR ${protobuffs.protobuffOutDir}\
      --prefix PATH ${
        lib.makeBinPath [ xwayland protobuf ]
      } # adds protobuffs to path
  '';
}
