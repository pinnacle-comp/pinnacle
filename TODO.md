- Provide scale and transform on new window/layer
    - AKA wl-compositor v6
- Properly stack x11 windows relative to each other
- Use env for snowcap socket dir
- Use xdg activation to activate new compositor-spawned windows
- Work on `ConnectorSavedState`
- Keyboard focus in Idea Xwayland is weird when creating a new Java file

- Snowcap crashes when a window opens and immediately closes because the foreign toplevel handle is no longer valid
- Cursor position when scaled is wrong

Testing
- Test layout mode changing and how it interacts with client fullscreen/maximized requests
    - Gonna need a test client for that
- Test `WindowHandle::in_direction`
- Test new output focus system

Problems:
- Pointer input to xwayland windows saturates at x=0, y=0, so windows on outputs at negative coords
  get screwed up pointer events
- Dragging an xwayland window to another output and closing a nested right click menu closes the whole
  right click menu because the keyboard focus is getting updated on the original output.
- Turning a monitor off then on causes scale increases to not propagate the new scale to clients resulting in blurry windows
