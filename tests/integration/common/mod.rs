use mlua::Lua;

pub mod client;
pub mod fixture;
pub mod server;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lang {
    Rust,
    Lua,
}

pub fn for_each_api(mut test: impl FnMut(Lang)) {
    test(Lang::Rust);
    test(Lang::Lua);
}

pub fn new_lua() -> Lua {
    let lua = unsafe { Lua::unsafe_new() };
    lua.load("Pinnacle = require('pinnacle')").exec().unwrap();
    lua.load("Input = require('pinnacle.input')")
        .exec()
        .unwrap();
    lua.load("Libinput = require('pinnacle.input.libinput')")
        .exec()
        .unwrap();
    lua.load("Process = require('pinnacle.process')")
        .exec()
        .unwrap();
    lua.load("Output = require('pinnacle.output')")
        .exec()
        .unwrap();
    lua.load("Tag = require('pinnacle.tag')").exec().unwrap();
    lua.load("Window = require('pinnacle.window')")
        .exec()
        .unwrap();
    lua.load("Layout = require('pinnacle.layout')")
        .exec()
        .unwrap();
    lua.load("Util = require('pinnacle.util')").exec().unwrap();

    lua
}
