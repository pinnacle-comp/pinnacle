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

> [!NOTE]
> All keyboard settings are currently global.

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

### XKB keymap

You can set an XKB keymap by providing a string of the keymap.
If you have a keymap in a file, read it into a string and provide that.

::: tabs key:langs
== Lua
```lua
require("pinnacle.input").set_xkb_keymap("keymap here...")

-- From a file
require("pinnacle.input").set_xkb_keymap(io.open("/path/to/keymap.xkb"):read("*a"))
```
== Rust
```rust
input::set_xkb_keymap("keymap here...");

// From a file
input::set_xkb_keymap(std::fs::read_to_string("/path/to/keymap.xkb")?);
```
:::

### Changing layouts

You can change keyboard layouts by either switching to one via index or
cycling them forward or backward.

::: tabs key:langs
== Lua
```lua
require("pinnacle.input").cycle_xkb_layout_forward()
require("pinnacle.input").cycle_xkb_layout_backward()
require("pinnacle.input").switch_xkb_layout(2)
```
== Rust
```rust
input::cycle_xkb_layout_forward();
input::cycle_xkb_layout_backward();
input::switch_xkb_layout(2);
```
:::
