-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
--
-- SPDX-License-Identifier: MPL-2.0

---@meta _

---@class _Msg
---@field SetKeybind { key: Keys, modifiers: Modifier[], callback_id: integer }
---@field SetMousebind { button: integer }
--Windows
---@field CloseWindow { window_id: integer }
---@field ToggleFloating { window_id: integer }
---@field SetWindowSize { window_id: integer, width: integer?, height: integer? }
---@field MoveWindowToTag { window_id: integer, tag_id: string }
---@field ToggleTagOnWindow { window_id: integer, tag_id: string }
--
---@field Spawn { command: string[], callback_id: integer? }
---@field Request Request
--Tags
---@field ToggleTag { output_name: string, tag_name: string }
---@field SwitchToTag { output_name: string, tag_name: string }
---@field AddTags { output_name: string, tag_names: string[] }
---@field RemoveTags { output_name: string, tag_names: string[] }
---@field SetLayout { output_name: string, tag_name: string, layout: Layout }
--Outputs
---@field ConnectForAllOutputs { callback_id: integer }

---@alias Msg _Msg | "Quit"

--------------------------------------------------------------------------------------------

---@class _Request
--Windows
---@field GetWindowProps { window_id: WindowId }
--Outputs
---@field GetOutputProps { output_name: string }
--Tags
---@field GetTagProps { tag_id: TagId }

---@alias Request _Request | "GetWindows" | "GetOutputs" | "GetTags"

---@class IncomingMsg
---@field CallCallback { callback_id: integer, args: Args }
---@field RequestResponse { response: RequestResponse }

---@class Args
---@field Spawn { stdout: string?, stderr: string?, exit_code: integer?, exit_msg: string? }
---@field ConnectForAllOutputs { output_name: string }

---@alias WindowId integer
---@alias TagId integer
---@alias OutputName string

---@class RequestResponse
--Windows
---@field Window { window_id: WindowId|nil }
---@field Windows { window_ids: WindowId[] }
---@field WindowProps { size: integer[]?, loc: integer[]?, class: string?, title: string?, floating: boolean?, focused: boolean? }
--Outputs
---@field Output { output_name: OutputName? }
---@field Outputs { output_names: OutputName[] }
---@field OutputProps { make: string?, model: string?, loc: integer[]?, res: integer[]?, refresh_rate: integer?, physical_size: integer[]?, focused: boolean?, tag_ids: integer[]? }
--Tags
---@field Tags { tag_ids: TagId[] }
---@field TagProps { active: boolean?, name: string?, output_name: string? }
