# Binds

Pinnacle currently supports keybinds and mousebinds. Binds to gestures and touch are planned.

> [!TIP]
> The Lua API has a shorter, alternative way to bind keys and mouse buttons.
> See the [Lua reference](https://pinnacle-comp.github.io/lua-reference/) for more information.

## Modifiers

All binds can be gated behind a set of modifier keys that must be pressed to allow the bind to trigger.
For example, to require the super and shift modifiers for a bind, provide the following:

::: tabs key:langs
== Lua
```lua
{ "super", "shift" }
```
== Rust
```rust
Mod::SUPER | Mod::SHIFT
```
:::

### Ignoring modifiers

Normally, when modifiers are omitted from a bind, Pinnacle requires them to *not* be held down.
In the above example, the alt and ctrl keys must not be held for the bind to trigger.

However, you may want to relax that restriction. For instance, you may want to bind `ctrl` and the `super` key itself.
If you passed no modifiers, the bind would fail because the super modifier is held when you press the super key.
To ignore any modifiers from being checked, pass in the corresponding "ignore" modifier:

::: tabs key:langs
== Lua
```lua
{ "ctrl", "ignore_super" }
```
== Rust
```rust
Mod::CTRL | Mod::IGNORE_SUPER
```
:::

## Builtin bind actions

Because all binds execute user-provided functions, deadlocks and blocks in your config prevent them from running.
This is bad, because it prevents you from quitting Pinnacle or reloading the config.

In order to work around this, there are two builtin bind actions that you can set any bind to trigger:
1. Quit, and
2. Reload config.

::: tabs key:langs
== Lua
```lua
require("pinnacle.input").keybind({
    -- Other keybind options
    quit = true,
    -- Or
    reload_config = true,
})
```
== Rust
```rust
pinnacle_api::input::keybind(...)
    .set_as_quit()
    // Or
    .set_as_reload_config();
```
:::

When set on a bind, Pinnacle will not call out to the config to execute anything. Rather, it quits or
reloads the config immediately when the bind triggers, sidestepping the config. This allows the action
to run no matter what state your config is in.

## Bind groups and descriptions

In order to provide a bind overlay that shows you what binds you have and what they do,
you can provide a group and/or description to any bind. The group groups binds (duh), and
the description tells you what the bind does.

::: tabs key:langs
== Lua
```lua
require("pinnacle.input").keybind({
    group = "Compositor",
    description = "Quits Pinnacle",
})
```
== Rust
```rust
pinnacle_api::input::keybind(...)
    .group("Compositor")
    .description("Quits Pinnacle");
```
:::

## Bind layers

Bind layers, also known as bind modes, allow you to group binds together and enable only the ones
on the bind layer you are on. Internally, bind layers are string identifiers.

### Creating binds on a layer

To create a bind on a layer, do the following:

::: tabs key:langs
== Lua
```lua
require("pinnacle.input").keybind({
    -- Other keybind options
    bind_layer = "bind_layer_name_here",
})
```
== Rust
```rust
BindLayer::get("bind_layer_name_here").keybind(...);
```
:::

The default bind layer is the one with a null string identifier.

To enter a bind layer:

::: tabs key:langs
== Lua
```lua
require("pinnacle.input").enter_bind_layer("bind_layer_name_here")
```
== Rust
```rust
BindLayer::get("bind_layer_name_here").enter();
```
:::

You will want to bind something that will return to the default layer,
or else you'd get stuck in the new layer.

::: tabs key:langs
== Lua
```lua
require("pinnacle.input").enter_bind_layer(nil)
```
== Rust
```rust
BindLayer::DEFAULT.enter();
```
:::

## Allowing binds when locked

You can allow binds to trigger when the session is locked using the `allow_when_locked` option.

::: tabs key:langs
== Lua
```lua
require("pinnacle.input").keybind({
    -- Other keybind options
    allow_when_locked = true,
})
```
== Rust
```rust
input::keybind(...).allow_when_locked();
```
:::

## Keybinds

A keybind executes a function everytime the keybind is pressed (or released).

::: tabs key:langs
== Lua
```lua
require("pinnacle.input").keybind({
    mods = { "super", "shift" },
    key = "c",
    on_press = function()
        -- Do something here
    end,
    on_release = function()
        -- Do something here
    end,
})
```
== Rust
```rust
input::keybind(Mod::SUPER | Mod::SHIFT, 'c')
    .on_press(|| { /* Do something here */ })
    .on_release(|| { /* Do something here */ });
```
:::

## Mousebinds

A mousebind executes a function everytime a mouse button is pressed (or released).

::: tabs key:langs
== Lua
```lua
require("pinnacle.input").mousebind({
    mods = { "super", "shift" },
    button = "btn_right",
    on_press = function()
        -- Do something here
    end,
    on_release = function()
        -- Do something here
    end,
})
```
== Rust
```rust
input::mousebind(Mod::SUPER | Mod::SHIFT, MouseButton::Right)
    .on_press(|| { /* Do something here */ })
    .on_release(|| { /* Do something here */ });
```
:::
