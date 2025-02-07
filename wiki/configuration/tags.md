# Tags

Instead of workspaces, Pinnacle manages windows through a tag system.

Each output has a set of tags. Windows can be tagged with any number of those tags.
Tags are either active or inactive. Only windows tagged with an active tag
are shown on screen.

## Adding tags

To add tags on an output, call the `add` function, providing the names of the tags
to be added as well as the output. This function will return handles to all the
newly created tags.

::: tabs key:langs
== Lua
```lua
local output = require("pinnacle.output").get_by_name("eDP-1")
local tags = require("pinnacle.tag").add(output, { "1", "2", "3", "4", "5" })
```
== Rust
```rust
let output = output::get_by_name("eDP-1")?;
let tags = tag::add(&output, ["1", "2", "3", "4", "5"]);
```
:::

## Manipulating tags

You can manipulate tags through the tag handles returned by `add`. You can also call
`get_all` to get handles to all tags across all outputs, or `get` to get a tag on an output.
There are also methods on output and window handles that return their tags.

There are a few methods on tag handles that manipulate a tag's `active` state.

| Method | Action |
| ------ | ------ |
| `switch_to` | Sets all tags but this tag to inactive, and sets this tag to active. This emulates a traditional workspace. |
| `set_active`/`toggle_active` | Sets or toggles this tag's `active` state |

::: tabs key:langs
== Lua
```lua
local tag = require("pinnacle.tag").get("1")
tag:switch_to()
tag:set_active(true)
tag:toggle_active()
```
== Rust
```rust
let tag = tag::get("1")?;
tag.switch_to();
tag.set_active(true);
tag.toggle_active();
```
:::

## Tagging windows

When a window opens, it is automatically tagged with the active tags on
the focused output.

There are a few methods on windows that manipulate tags on them.

| Method | Action |
| ------ | ------ |
| `set_tag`/`toggle_tag` | Adds a tag to or removes it from this window |
| `move_to_tag` | Removes all tags from this window and adds the given one. This "moves" the window to the tag. |

> [!NOTE]
> The behavior when tagging a window with multiple tags from different outputs is currently
> undefined. Try not to do that.

::: tabs key:langs
== Lua
```lua
local window = require("pinnacle.window").get_focused()
local tag = require("pinnacle.tag").get("1")
window:set_tag(tag, true)
window:toggle_tag(tag)
window:move_to_tag(tag)
```
== Rust
```rust
let window = window::get_focused()?;
let tag = tag::get("1")?;
window.set_tag(&tag, true);
window.toggle_tag(&tag);
window.move_to_tag(&tag);
```
:::
