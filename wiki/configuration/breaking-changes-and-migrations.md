# Breaking changes and migrations

This page details breaking changes and steps to update your config
across major versions.

## v0.2.0

### Layout changes

- Tiled windows are now resizable! To be able to remember tile sizes,
  a unique ID now must be returned along with the computed layout node
  during layout requests.

  As a result, the return type of the function passed to `manage` has
  changed to a `LayoutResponse`, which contains an additional identifier:

  ::: tabs key:langs
  == Lua
  ```lua
  require("pinnacle.layout").manage(function(args)
      local layout_node = require("pinnacle.layout")
          .builtin
          .master_stack()
          :layout(args.window_count)

      return layout_node -- [!code --]
      ---@type pinnacle.layout.LayoutResponse # [!code ++]
      return { -- [!code ++]
          root_node = layout_node, -- [!code ++]
          tree_id = 0, -- [!code ++]
      } -- [!code ++]
  end)
  ```
  == Rust
  ```rust
  layout::manage(|args| {
      let layout_node = MasterStack::default().layout(args.window_count);

      layout_node // [!code --]
      LayoutResponse { // [!code ++]
          root_node: layout_node, // [!code ++]
          tree_id: 0, // [!code ++]
      } // [!code ++]
  });
  ```
  :::

  For those of you using the `Cycle` layout generator (used in the
  default config), a unique ID can be retrieved using
  `Cycle::current_tree_id`:

  ::: tabs key:langs
  == Lua
  ```lua
  require("pinnacle.layout").manage(function(args)
      local first_tag = args.tags[1]
      if not first_tag then
          ---@type pinnacle.layout.LayoutResponse
          return {
              root_node = {},
              tree_id = 0,
          }
      end
      layout_cycler.current_tag = first_tag
      local root_node = layout_cycler:layout(args.window_count)
      local tree_id = layout_cycler:current_tree_id() -- [!code highlight]

      ---@type pinnacle.layout.LayoutResponse
      return {
          root_node = root_node,
          tree_id = tree_id,
      }
  end)
  ```
  == Rust
  ```rust
  layout::manage({
      move |args| {
          let Some(tag) = args.tags.first() else {
              return LayoutResponse {
                  root_node: LayoutNode::new(),
                  tree_id: 0,
              };
          };

          let mut cycler = cycler.lock().unwrap();
          cycler.set_current_tag(tag.clone());

          let root_node = cycler.layout(args.window_count);
          let tree_id = cycler.current_tree_id(); // [!code highlight]
          LayoutResponse { root_node, tree_id }
      }
  });
  ```
  :::

  It is *highly* recommended to use this unique ID to let Pinnacle
  remember tile sizes across different tags and layouts.


### Focus changes

- The focused output is no longer determined by pointer motion.
  With a non-updated config, you will need to click an output to focus it 
  and have windows open on it.

  To restore the old behavior, connect to the new `OutputPointerEnter`
  signal and use it to focus outputs:

  ::: tabs key:langs
  == Lua
  ```lua
  require("pinnacle.output").connect_signal({
      pointer_enter = function(output)
          output:focus()
      end
  })
  ```
  == Rust
  ```rust
  output::connect_signal(OutputSignal::PointerEnter(Box::new(|output| {
      output.focus();
  })));
  ```
  :::

- Window focus is now updated based on the focused output.
  This means focusing a different output will update the window focus
  to a window on that output. For example, with the above signal 
  connection, moving the pointer to an empty output will unfocus the 
  currently focused window if it doesn't overlap the new output. This
  behavior mirrors Sway more than it does Awesome, and this may be changed
  to be closer to the way Awesome behaves in the future.


### Other API changes

#### Rust

- `FocusBorder::decorate` now returns a result. It is possible for the
  passed window to not exist. Previously, this would panic. This is no
  longer the case.

## v0.1.0

No breaking changes
