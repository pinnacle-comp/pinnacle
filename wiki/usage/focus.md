# Focus

**Focus** primarily determines the window that keyboard input goes to.
There are two types of focus: window and output, discussed below.

## Window focus

Window focus is the "main" focus (the one that actually determines
keyboard focus). The focused window receives all keyboard input apart
from keybinds.

To change the focused window, either: 
1. Call `WindowHandle::set_focused`,
2. Call `WindowHandle::toggle_focused`, or
3. Click on a window.

Directional focus can be achieved by getting windows in a direction
with `WindowHandle::in_direction` and focusing one of them.

## Output focus

Output focus determines which set of windows can actually be focused.
Only windows that overlap the focused output are eligible to gain
window focus. Additionally, any windows that open will open on the focused output.

To change the focused output, either:
1. Call `OutputHandle::focus`, or
2. Click on an output.

Directional focus can be achieved by getting outputs in a direction with
`OutputHandle::in_direction` and focusing one of them.

> [!NOTE]
> The focused output *also* changes when focusing a window that doesn't
> overlap the currently focused output.
