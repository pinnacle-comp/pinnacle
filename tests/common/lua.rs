use std::{
    io::Write,
    process::{Command, Stdio},
    sync::LazyLock,
};

use anyhow::anyhow;
use mlua::{AsChunk, Lua};

pub static LUA: LazyLock<Lua> = LazyLock::new(new_lua);

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

#[macro_export]
macro_rules! run_lua {
    { $($body:tt)* } => {{
        $crate::common::lua::LUA.load(::mlua::chunk! {
            Pinnacle.run(function()
                local run = function()
                    $($body)*
                end

                local success, err = pcall(run)

                if not success then
                    error(err)
                end
            end)
        }).exec().map_err(|err| ::anyhow::anyhow!("{err}"))
    }};
}

#[macro_export]
macro_rules! setup_lua {
    { $($body:tt)* } => {{
        ::std::thread::spawn(move || {
            let lua = $crate::common::lua::new_lua();
            let task = lua.load(::mlua::chunk! {
                Pinnacle.setup(function()
                    local run = function()
                        $($body)*
                    end

                    local success, err = pcall(run)

                    if not success then
                        error(err)
                    end
                end)
            });

            task.exec().unwrap();
        });
    }};
}
