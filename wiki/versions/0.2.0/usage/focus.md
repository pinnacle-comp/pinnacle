# Focus

**Focus** determines the application that keyboard input goes to
(there is also the concept of pointer focus, but "focus" in
this page primarily discusses keyboard focus).
There are two types of focus: keyboard and output, discussed below.

## Keyboard focus

Keyboard focus is the "main" focus. There are four types of focusable targets:

1. Windows
2. Popups
3. Layer-shell surfaces
4. Lock screen surfaces

These surfaces are focusable in different ways (subject to change).

### Focusing windows

Windows can be focused by either:

1. Clicking on them,
2. Calling `WindowHandle::set_focused`,
3. Calling `WindowHandle::toggle_focused`, or
4. Focusing a different output.

Directional focus can be achieved by getting windows in a direction
with `WindowHandle::in_direction` and focusing one of them.

### Focusing popups

Popups automatically gain keyboard focus when they appear;
no action is necessary.

### Focusing layer-shell surfaces

Layer-shell surfaces are surfaces created using the `wlr-layer-shell` protocol.
They are usually used for things like taskbars, notifications,
and launchers.

A layer-shell surface can only be focused if it sets its
keyboard interactivity to either `on_demand` or `exclusive`.

Layer-shell surfaces with `exclusive` keyboard interactivity will
automatically gain keyboard focus. If there are multiple `exclusive`
surfaces, the top-most one gets focus, prioritizing those on the
focused output.

To focus a layer-shell surface with `on_demand` interactivity,
click on it.

### Focusing lock screen surfaces

Some applications allow you to lock the session, displaying lock
screen surfaces that let you input a password to unlock.

A lock surface will be automatically focused once it appears.
To change the focused lock surface, click on a different one.

## Output focus

Output focus determines which set of windows can actually be focused.
Only windows that overlap the focused output are eligible to gain
keyboard focus. Additionally, any windows that open will open on the focused output.

To change the focused output, either:
1. Call `OutputHandle::focus`, or
2. Click on an output.

Directional focus can be achieved by getting outputs in a direction with
`OutputHandle::in_direction` and focusing one of them.

> [!NOTE]
> The focused output *also* changes when focusing a window that doesn't
> overlap the currently focused output.
