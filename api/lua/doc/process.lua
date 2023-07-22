-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
--
-- SPDX-License-Identifier: MPL-2.0

---This file houses LDoc documentation with dummy functions.---

---Process management
---@module ProcessModule
local process_module = {}

---Spawn a process with an optional callback for its stdout, stderr, and exit information.
---
---`callback` has the following parameters:<br>
--- - `stdout` - The process's stdout printed this line.<br>
--- - `stderr` - The process's stderr printed this line.<br>
--- - `exit_code` - The process exited with this code.<br>
--- - `exit_msg` - The process exited with this message.<br>
---@tparam string|string[] command The command as one whole string or a table of each of its arguments
---@tparam function callback A callback to do something whenever the process's stdout or stderr print a line, or when the process exits.
function process_module.spawn(command, callback) end

---Spawn a process only if it isn't already running, with an optional callback for its stdout, stderr, and exit information.
---
---`callback` has the following parameters:<br>
--- - `stdout` - The process's stdout printed this line.<br>
--- - `stderr` - The process's stderr printed this line.<br>
--- - `exit_code` - The process exited with this code.<br>
--- - `exit_msg` - The process exited with this message.<br>
---@tparam string|string[] command The command as one whole string or a table of each of its arguments
---@tparam function callback A callback to do something whenever the process's stdout or stderr print a line, or when the process exits.
function process_module.spawn_once(command, callback) end
