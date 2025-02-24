- Provide scale and transform on new window/layer
    - AKA xdg-shell v6

- Properly stack x11 windows relative to each other
- Use env for snowcap socket dir
- Don't log to local/state

- Streamline handling of unmapped windows
- Use xdg activation to activate new compositor-spawned windows
- Work on `ConnectorSavedState`

Problems:
- Pointer input to xwayland windows saturates at x=0, y=0, so windows on outputs at negative coords
  get screwed up pointer events
- Dragging an xwayland window to another output and closing a nested right click menu closes the whole
  right click menu because the keyboard focus is getting updated on the original output.
- Turning a monitor off then on causes scale increases to not propagate the new scale to clients resulting in blurry windows
