- Provide scale and transform on new window/layer
    - AKA xdg-shell v6
- Properly stack x11 windows relative to each other
- Use env for snowcap socket dir
- Don't log to local/state
- Use xdg activation to activate new compositor-spawned windows
- Work on `ConnectorSavedState`
- Remove startup logs and globals print for CLI client
- Refactor handling of client vs config window rules
    - Track which is which eg whether a window's fullscreen mode is from the config
      or the client
- Encode window rules in an enum type for pre- and post-initial configure
- Lua reference seems to be missing some `?`s in places like return types
- PROBLEM: I think `map_new_window` doesn't map maximized/fullscreen windows

Problems:
- Pointer input to xwayland windows saturates at x=0, y=0, so windows on outputs at negative coords
  get screwed up pointer events
- Dragging an xwayland window to another output and closing a nested right click menu closes the whole
  right click menu because the keyboard focus is getting updated on the original output.
- Turning a monitor off then on causes scale increases to not propagate the new scale to clients resulting in blurry windows
