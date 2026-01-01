# Config Basics

While some compositors expose configuration through a declarative file or shell commands, Pinnacle instead provides
configuration libraries used by config clients. There are both Lua and Rust libraries for you to pick from.

I say config *client* because that is exactly what a Pinnacle config is. All configuration,
from keybinds to output management, is exposed as a gRPC interface over a Unix domain socket.
A config is a program that utilizes the Lua or Rust API to communicate with Pinnacle's gRPC server.

Let's take a look at what the config generator created.

## The `pinnacle.toml` file

Pinnacle, apart from the builtin Rust config, *does not know* about the Lua or Rust libraries.
That is to say, it won't try to look for something like an `init.lua` in your config directory
or attempt to execute `cargo run`. Indeed, you could, if you had way too much free time, create a
Python gRPC library for Pinnacle and use that. Instead of hardcoding the ways to start a config,
Pinnacle instead relies on you to provide a command that does that.
It does this through a `pinnacle.toml` file. This file must exist in your config directory,
otherwise Pinnacle will fall back to the builtin config.

In your config directory, open the generated `pinnacle.toml` file. You should see the following field:

:::tabs key:langs
== Lua
```toml
run = ["lua", "default_config.lua"]
```
== Rust
```toml
run = ["cargo", "run"]
```
:::

The `run` field declares the command that Pinnacle will run to start your config.
For a Lua config, it spins up an instance of Lua. For a Rust config, it delegates to Cargo.

### Other Fields

In addition to `run`, there are a few other fields that you can set:
| Name | Type | Description |
| ---- | ---- | ----------- |
| `socket_dir` | string | Sets the directory Pinnacle will open the gRPC socket at |
| `envs` | table | A table of key-value fields denoting the environment variables Pinnacle will spawn the config with |
| `no_xwayland` | bool | Prevents xwayland from starting |
| `no_config` | bool | Prevents the config from starting (aka stops `run` from running) |

## The actual config

Now that we've looked at how your config starts, let's get to the meat and potatoes: the actual config!

The config generator should have, in addition to the `pinnacle.toml` file, generated either `default_config.lua`
or a small Cargo project depending on which language you chose.

Let's look at the main config file. You'll see that the config must define an "entry point"—a place where
API calls are valid:

:::tabs key:langs
== Lua
```lua 
local Pinnacle = require("pinnacle")
Pinnacle.setup(function()
    -- All the config stuff here
end)
```
== Rust
```rust
async fn config() {
    // All the config stuff here
}
pinnacle_api::main!(config);
```
:::

The entry point connects to Pinnacle's gRPC server, calls your config function, and blocks to execute
incoming and outgoing gRPC requests and replies. Any API calls made before the entry point will fail.

## On crashing and deadlocks

Being a program that you write, it's possible for your config to crash or deadlock. A crash is recoverable;
Pinnacle will simply start up the builtin config, and you can restart your config from there.

Deadlocks are more of an issue, however. If your config is deadlocked or blocked in any way, it won't
be able to run keybinds or respond to compositor requests. Notably, this may prevent you from being able to
exit Pinnacle or reload your config. There are ways around this, discussed later in [Binds](./binds), but
in the general case you may have to switch TTYs and kill Pinnacle.
