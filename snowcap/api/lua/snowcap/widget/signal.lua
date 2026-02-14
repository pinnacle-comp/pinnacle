-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.

---Signals emitted by widgets.
---
---@enum snowcap.widget.signal
return {
    ---Notifies that a redraw is needed.
    redraw_needed = "widget::redraw_needed",
    ---Emits a message that will update widgets.
    send_message = "widget::send_message",
    ---Notifies that a widget closed.
    closed = "widget::closed",

    ---Update widgets' internal state.
    operation = "widget::operation",
}
