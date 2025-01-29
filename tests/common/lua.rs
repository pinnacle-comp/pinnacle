use std::{
    io::Write,
    process::{Command, Stdio},
};

use anyhow::anyhow;

pub fn run_lua(code: &str) -> anyhow::Result<()> {
    #[rustfmt::skip]
    let code = format!(r#"
        local Pinnacle = require("pinnacle")
        local Input = require("pinnacle.input")
        local Process = require("pinnacle.process")
        local Output = require("pinnacle.output")
        local Tag = require("pinnacle.tag")
        local Window = require("pinnacle.window")
        local Render = require("pinnacle.render")
        local Layout = require("pinnacle.layout")

        require("pinnacle").run(function()
            local run = function()
                {code}
            end

            local success, err = pcall(run)

            if not success then
                print(err)
                print("exiting")
                os.exit(1)
            end
        end)
    "#);

    let mut child = Command::new("lua").stdin(Stdio::piped()).spawn()?;

    let mut stdin = child.stdin.take().ok_or(anyhow!("child had no stdin"))?;

    stdin.write_all(code.as_bytes())?;

    drop(stdin);

    let exit_status = child.wait()?;

    if exit_status.code().is_some_and(|code| code != 0) {
        return Err(anyhow!("lua code panicked"));
    }

    Ok(())
}

#[allow(dead_code)] // TODO:
pub struct SetupLuaGuard {
    child: std::process::Child,
}

impl Drop for SetupLuaGuard {
    fn drop(&mut self) {
        let _ = self.child.kill();
    }
}

#[allow(dead_code)] // TODO:
pub fn setup_lua(code: &str) -> anyhow::Result<SetupLuaGuard> {
    #[rustfmt::skip]
    let code = format!(r#"
        require("pinnacle").setup(function()
            local run = function()
                {code}
            end

            local success, err = pcall(run)

            if not success then
                print(err)
                print("exiting")
                os.exit(1)
            end
        end)
    "#);

    let mut child = Command::new("lua").stdin(Stdio::piped()).spawn()?;

    let mut stdin = child.stdin.take().ok_or(anyhow!("child had no stdin"))?;

    stdin.write_all(code.as_bytes())?;

    drop(stdin);

    Ok(SetupLuaGuard { child })
}

#[macro_export]
macro_rules! run_lua {
    { $($body:tt)* } => {
        $crate::common::lua::run_lua(stringify!($($body)*))?;
    };
}

#[allow(unused_macros)] // TODO:
macro_rules! setup_lua {
    { $($body:tt)* } => {
        let _guard = $crate::common::lua::setup_lua(stringify!($($body)*))?;
    };
}
