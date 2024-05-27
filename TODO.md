- Re-add raising file descriptor limit
    - Like an idiot I managed to remove that sometime and not add it back
- Provide scale and transform on new window/layer

Problems:
- Pointer input to xwayland windows saturates at x=0, y=0, so windows on outputs at negative coords
  get screwed up pointer events
- Xwayland popups are screwed when the output is not at (0, 0)
- Dragging an xwayland window to another output and closing a nested right click menu closes the whole
  right click menu because the keyboard focus is getting updated on the original output.
- Transactions don't render floating windows
