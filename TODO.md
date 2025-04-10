- Provide scale and transform on new window/layer
    - AKA wl-compositor v6
- Properly stack x11 windows relative to each other
- Use env for snowcap socket dir
- Don't log to local/state
- Use xdg activation to activate new compositor-spawned windows
- Work on `ConnectorSavedState`
- Remove startup logs and globals print for CLI client
- Encode window rules in an enum type for pre- and post-initial configure
- Keyboard focus in Idea Xwayland is weird when creating a new Java file
- Spawn unique when running `with_shell` using `systemd-run` doesn't dedup because systemd-run
  renames the command to its full path

Testing
- Test layout mode changing and how it interacts with client fullscreen/maximized requests
    - Gonna need a test client for that

Problems:
- Pointer input to xwayland windows saturates at x=0, y=0, so windows on outputs at negative coords
  get screwed up pointer events
- Dragging an xwayland window to another output and closing a nested right click menu closes the whole
  right click menu because the keyboard focus is getting updated on the original output.
- Turning a monitor off then on causes scale increases to not propagate the new scale to clients resulting in blurry windows
