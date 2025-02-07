# Xwayland

Pinnacle integrates Xwayland into the compositor to allow legacy X11 windows to work.

## Scaling

X11 does not handle scaling well. As a result, on outputs with a scale above 1,
X11 windows will look blurry. To mitigate this, you can tell X11 clients to ignore Wayland scaling
and attempt to scale themselves using `set_xwayland_self_scaling`.

::: tabs key:langs
== Lua
```lua
require("pinnacle").set_xwayland_self_scaling(true)
```
== Rust
```rust
pinnacle::set_xwayland_self_scaling(true);
```
:::

If the application does not support scaling, it will render as if the output had a scale of 1.

If you do not want to do that, you can instead change the upscale filter to `nearest_neighbor`
to make blurry windows pixelated instead.

::: tabs key:langs
== Lua
```lua
require("pinnacle.render").set_upscale_filter("nearest_neighbor")
```
== Rust
```rust
render::set_upscale_filter(ScalingFilter::NearestNeighbor);
```
:::
