# Outputs

An output is the Wayland term for a monitor. They display what the compositor renders.

Outputs are mapped to a coordinate in a global space.

## Output setup

You can set up outputs using the `for_each_output` function. This function runs a closure
on all currently connected outputs as well as all newly connected ones.

::: tabs key:langs
== Lua
```lua
require("pinnacle.output").for_each_output(function(output)
    -- Do stuff with output
end)
```
== Rust
```rust
output::for_each_output(|output| {
    // Do stuff with output
});
```
:::

### Setting up tags

You will want to add tags to all outputs you have, or else they won't
be able to display windows.

> [!IMPORTANT]
> Currently, adding tags to an output doesn't set any of them to active.
> You will want to make at least one tag active. This will most likely change
> in the future.

::: tabs key:langs
== Lua
```lua
require("pinnacle.output").for_each_output(function(output)
    local tags = require("pinnacle.tag").add(output, { "1", "2", "3" })
    tags[1]:set_active(true)
end)
```
== Rust
```rust
output::for_each_output(|output| {
    let mut tags = tag::add(output, ["1", "2", "3"]);
    tags.next()?.set_active(true);
});
```
:::

### Setting output locations

For those with multiple monitors, you may want to change the locations of outputs.

The output API provides a lower-level way to move outputs through `set_loc`.
This directly moves the output to the given coordinates in the global space.

::: tabs key:langs
== Lua
```lua
local output = require("pinnacle.output").get_focused()
output:set_loc(1920, 0)
```
== Rust
```rust
let output = output::get_focused()?;
output.set_loc(1920, 0)
```
:::

> [!NOTE]
> If you move outputs such that there is a gap between them, the pointer
> will not be able to move from one output to the other.

Of course, setting the location like this is error-prone.
A helper function is provided to move outputs relative to other outputs: `set_loc_adj_to`.

`set_loc_adj_to` moves an output adjacent to another output. You can specify
which side to move it adjacent to and how to align the outputs.

::: tabs key:langs
== Lua
```lua
local hdmi1 = require("pinnacle.output").get_by_name("HDMI-1")
local dp1 = require("pinnacle.output").get_by_name("DP-1")
hdmi1:set_loc_adj_to(dp1, "left_align_bottom")
```
== Rust
```rust
let hdmi1 = output::get_by_name("HDMI-1")?;
let dp1 = output::get_by_name("DP-1")?;
hdmi1.set_loc(dp1, Alignment::LeftAlignBottom);
```
:::

> [!TIP]
> You may opt to use external tools like [kanshi](https://sr.ht/~emersion/kanshi/)
> to simplify the process of laying out outputs, especially when your monitor setup
> changes frequently.

### Variable refresh rate

Variable refresh rate (VRR), also known as adaptive sync, allows supported monitors to refresh
below the maximum refresh rate. This prevents stuttering for applications like games that
may not be able to hit FPS that matches the refresh rate of the display.

Outputs have three VRR modes:

1. **Off**

   This disables VRR on the output.

   ::: tabs key:langs
   == Lua
   ```lua
   require("pinnacle.output").get_focused():set_vrr(false)
   ```
   == Rust
   ```rust
   output::get_focused().set_vrr(Vrr::Off);
   ```
   :::

2. **Always-On**

   This enables VRR on the output at all times.

   ::: tabs key:langs
   == Lua
   ```lua
   require("pinnacle.output").get_focused():set_vrr(true)
   ```
   == Rust
   ```rust
   output::get_focused().set_vrr(Vrr::AlwaysOn);
   ```
   :::

3. **On-Demand**

   This only enables VRR on the output while a window with an active VRR demand is visible.

   ::: tabs key:langs
   == Lua
   ```lua
   require("pinnacle.output").get_focused():set_vrr("on_demand")
   ```
   == Rust
   ```rust
   output::get_focused().set_vrr(Vrr::OnDemand);
   ```
   :::

   To set a window's VRR demand, call `set_vrr_demand`. A window can have either no demand,
   which prevents it from turning on VRR, or one of the following two demands:

   1. When visible - The window will turn on VRR when it is visible anywhere on the output, or
   2. When fullscreen - The window will turn on VRR when it is both visible *and* fullscreen
      on the output.

   ::: tabs key:langs
   == Lua
   ```lua
   require("pinnacle.window").get_focused():set_vrr_demand(nil)
   require("pinnacle.window").get_focused():set_vrr_demand("visible")
   require("pinnacle.window").get_focused():set_vrr_demand("fullscreen")
   ```
   == Rust
   ```rust
   window::get_focused().set_vrr_demand(None);
   window::get_focused().set_vrr_demand(VrrDemand::when_visible());
   window::get_focused().set_vrr_demand(VrrDemand::when_fullscreen());
   ```
   :::
