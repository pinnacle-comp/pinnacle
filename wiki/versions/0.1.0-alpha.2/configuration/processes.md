# Processes

The process API allows you to spawn processes, capture their output, and wait for them to exit.

## Spawning

To spawn a process, create a `Command` and `spawn` it.

::: tabs key:langs
== Lua
```lua
require("pinnacle.process").command({
    cmd = "alacritty",
}):spawn()
-- Or
require("pinnacle.process").spawn("alacritty")
```
== Rust
```rust
Command::new("alacritty").spawn();
```
:::

`Command`s have the following properties:

| Property | Type | Description |
| -------- | ---- | ----------- |
| Command | String or String[] | The command and its arguments |
| Shell command | String[] | A shell and arguments that gets prepended to the command above |
| Envs | Map\<String, String> | Environment variables to spawn the command with |
| Unique | Bool | Causes the command to not spawn if an instance of it is already running |
| Once | Bool | Causes the command to not spawn it has been spawned at any time during the current session |

### Special spawn options

#### Unique

To prevent multiple instances of a process from spawning, use the `unique` flag.

::: tabs key:langs
== Lua
```lua
require("pinnacle.process").command({
    cmd = "alacritty",
    unique = true,
}):spawn()
-- Or
require("pinnacle.process").spawn_unique("alacritty")
```
== Rust
```rust
Command::new("alacritty").unique().spawn();
```
:::

#### Once

To only spawn a process exactly once, use the `once` flag. This is useful for startup programs that you close
and don't want respawning when reloading the config.

::: tabs key:langs
== Lua
```lua
require("pinnacle.process").command({
    cmd = "alacritty",
    once = true,
}):spawn()
-- Or
require("pinnacle.process").spawn_once("alacritty")
```
== Rust
```rust
Command::new("alacritty").once().spawn();
```
:::

#### With shell

Sometimes you may want to spawn something using a shell, for example to enable piping.
Using the shell command feature allows you to do this while also allowing `once` and
`unique` to only process the actual command (if you pass the shell as the actual command it will trigger `once` and `unique`).

::: tabs key:langs
== Lua
```lua
require("pinnacle.process").command({
    cmd = "echo hello | cat",
    shell_cmd = { "bash", "-c" },
}):spawn()
```
== Rust
```rust
Command::with_shell(["bash", "-c"], "echo hello | cat").spawn();
```
:::

## Capturing output

If the command spawns successfully and has standard IO, a `Child` object will be returned with the process's
`stdin`, `stdout`, and `stderr`.

::: tabs key:langs
== Lua

<div class="pad-content">

You can run a closure on every outputted line from `stdout` and `stderr` by calling the appropriate method:

</div>

```lua
local child = require("pinnacle.process").spawn("alacritty")
if child then
    child:on_line_stdout(function(line)
        print("stdout: " .. line)
    end)
    child:on_line_stderr(function(line)
        print("stderr: " .. line)
    end)
end
```
== Rust

<div class="pad-content">

The child contains `tokio::process::ChildStd{in,out,err}`.
You can wrap the out and err with tokio's `BufReader` to
do stuff with the lines.

</div>

```rust
let child = Command::new("alacritty").spawn()?;
if let Some(stdout) = child.stdout.take() {
    let mut lines = tokio::io::BufReader::new(stdout).lines();
    tokio::spawn(async move {
        while let Ok(Some(line)) = lines.next_line() {
            println!("stdout: {line}");
        }
    });
}
```
:::

## Waiting for the process

You can block and wait for the process to exit, additionally capturing its exit code and message.

::: tabs key:langs
== Lua
```lua
local child = require("pinnacle.process").spawn("alacritty")
local exit_info = child:wait()
```
== Rust
```rust
let child = Command::new("alacritty").spawn()?;
let exit_info = child.wait();
```
:::
