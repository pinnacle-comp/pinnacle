# Snowcap widgets

[Snowcap](https://github.com/pinnacle-comp/pinnacle/tree/main/snowcap) is a very,
*very* WIP widget system I'm building for Pinnacle.

It is currently used to render the bind overlay and quit prompt.

## Bind overlay

The bind overlay displays all bindings set along with their descriptions, grouped by their group.

![Bind overlay](/assets/bind_overlay.png)

To show the bind overlay, do the following:

::: tabs key:langs
== Lua
```lua
require("pinnacle.snowcap").integration.bind_overlay():show()
```
== Rust
```rust
pinnacle_api::snowcap::BindOverlay::new().show();
```
:::


## Quit prompt

The quit prompt asks you to press ENTER before Pinnacle quits.

![Quit prompt](/assets/quit_prompt.png)

To show the quit prompt, do the following:

::: tabs key:langs
== Lua
```lua
require("pinnacle.snowcap").integration.quit_prompt():show()
```
== Rust
```rust
pinnacle_api::snowcap::QuitPrompt::new().show();
```
:::

---

Both widgets have a few knobs that can be set. See the API reference for details.
