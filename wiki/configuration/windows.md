# Windows

Window management is a key part of any Wayland compositor. The window API
provides ways to manage windows, like fullscreening them, closing them, and more.

## Window Handles

Getting information about a window and controlling it happens through a window handle.
You can get a window handle through various means, like the `get_all` function, signals,
or window rules.

## Methods

Documentation for methods on window handles can be found at the corresponding API reference.

## Window rules

Unlike AwesomeWM and most Wayland compositors out there, Pinnacle does not have a declarative window rule system.
That is, you don't specify a list of conditions and applied rules directly. We do this because it introduces a split
between window rules and the rest of the window API. Instead, window rules give you a window handle when
a window opens that you can do whatever you want on.

To add a window rule, call `add_window_rule`.

::: tabs key:langs
== Lua
```lua
require("pinnacle.window").add_window_rule(function(window)
    if window:app_id() == "alacritty" then
        window:set_floating(true)
    end
end)
```
== Rust
```rust
window::add_window_rule(|window| {
    if window.app_id() == "alacritty" {
        window.set_floating(true)
    }
});
```
:::

> [!IMPORTANT]
> Try not to block inside of the window rule closure. Pinnacle will only configure the window once
> all window rules have finished executing. If you block here, there will be a delay before the window opens.
> If you deadlock here, the window will not open at all.
