use mlua::{Lua, Variadic};

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

    let override_print = lua
        .create_function(|_, strings: Variadic<String>| {
            let string = strings.join("");

            match string.split_once(" ") {
                Some(("INFO", msg)) => tracing::info!("{}", msg),
                Some(("DEBUG", msg)) => tracing::debug!("{}", msg),
                Some(("WARN", msg)) => tracing::warn!("{}", msg),
                Some(("ERROR", msg)) => tracing::error!("{}", msg),
                Some((level, msg)) => tracing::info!("{} {}", level, msg),
                None => tracing::info!("{}", string),
            }

            Ok(())
        })
        .unwrap();

    lua.globals().set("print", override_print).unwrap();

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
