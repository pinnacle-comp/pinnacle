# Input Devices

Pinnacle provides ways to manage input devices, like keyboards, mice, touchpads, and more.

## Device handles

Input devices are exposed to you through device handles. Handles can be retrieved by
requesting all of them manually, and they are also provided through the "device added" signal
and `for_each_device` function.

::: tabs key:langs
== Lua
```lua
local devices = require("pinnacle.input.libinput").get_devices()
```
== Rust
```rust
let devices = input::libinput::get_devices();
```
:::

Device handles expose ways to set various
[libinput](https://wayland.freedesktop.org/libinput/doc/latest/index.html) settings as well as getters
for device information, like name and vendor ID.

## Device setup

You will probably want to set device settings on both startup and whenever a new device is connected.
The API provides the `for_each_device` function, which allows you to supply a function that operates on
all currently connected input devices as well as all newly connected ones.

::: tabs key:langs
== Lua
```lua
require("pinnacle.input.libinput").for_each_device(function(device)
    if device:device_type() == "touchpad" then
        device:set_natural_scroll(true)
        device:set_tap(true)
    end
    -- Do other stuff with the device
end)
```
== Rust
```rust
input::libinput::for_each_device(|device| {
    if device.device_type().is_touchpad() {
        device.set_natural_scroll(true);
        device.set_tap(true);
    }
    // Do other stuff with the device
});
```
:::

Read the corresponding API reference to see all possible settings.

## Keyboard settings

Keyboards have some extra settings separate from libinput.

### xkeyboard-config

You can set a custom xkb-config by doing the following:

::: tabs key:langs
== Lua
```lua
require("pinnacle.input").set_xkb_config({
    layout = "us,fr,ge",
    options = "ctrl:swapcaps,caps:shift"
})
```
== Rust
```rust
input::set_xkb_config(XkbConfig::new()
    .with_layout("us,fr,ge")
    .with_options("ctrl:swapcaps,caps:shift"));
```
:::

This currently only supports setting the xkb-config globally.

### Repeat rate and delay

Setting the repeat rate and delay changes how long it takes a held down
key to start repeating as well as how often it repeats once it does.

::: tabs key:langs
== Lua
```lua
require("pinnacle.input").set_repeat_rate(25, 500)
```
== Rust
```rust
input::set_repeat_rate(25, 500);
```
:::

This currently only supports setting the repeat rate and delay globally.
