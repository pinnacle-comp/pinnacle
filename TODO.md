- Provide scale and transform on new window/layer

- Reloading config causes the next spawned process to not layout (off by one error?)
- Lua API doesn't call wait on processes = defuncts everywhere, change waiting api to not require a call to `wait`

Problems:
- Pointer input to xwayland windows saturates at x=0, y=0, so windows on outputs at negative coords
  get screwed up pointer events
- Xwayland popups are screwed when the output is not at (0, 0)
- Dragging an xwayland window to another output and closing a nested right click menu closes the whole
  right click menu because the keyboard focus is getting updated on the original output.
- Turning a monitor off then on causes scale increases to not propagate the new scale to clients resulting in blurry windows
