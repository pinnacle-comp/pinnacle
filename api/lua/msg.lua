-- SPDX-License-Identifier: GPL-3.0-or-later

---@meta _

---@class _Msg
---@field SetKeybind { key: { Int: Keys?, String: string? }, modifiers: Modifier[], callback_id: integer }?
---@field SetMousebind { modifiers: (Modifier)[], button: integer, edge: "Press"|"Release", callback_id: integer }?
--Windows
---@field CloseWindow { window_id: WindowId }?
---@field SetWindowSize { window_id: WindowId, width: integer?, height: integer? }?
---@field MoveWindowToTag { window_id: WindowId, tag_id: TagId }?
---@field ToggleTagOnWindow { window_id: WindowId, tag_id: TagId }?
---@field ToggleFloating { window_id: WindowId }?
---@field ToggleFullscreen { window_id: WindowId }?
---@field ToggleMaximized { window_id: WindowId }?
---@field AddWindowRule { cond: _WindowRuleCondition, rule: _WindowRule }?
---@field WindowMoveGrab { button: integer }?
---@field WindowResizeGrab { button: integer }?
--
---@field Spawn { command: string[], callback_id: integer? }?
---@field SetEnv { key: string, value: string }?
--Tags
---@field ToggleTag { tag_id: TagId }?
---@field SwitchToTag { tag_id: TagId }?
---@field AddTags { output_name: string, tag_names: string[] }?
---@field RemoveTags { tag_ids: TagId[] }?
---@field SetLayout { tag_id: TagId, layout: Layout }?
--Outputs
---@field ConnectForAllOutputs { callback_id: integer }?
---@field SetOutputLocation { output_name: OutputName, x: integer?, y: integer? }?
--Input
---@field SetXkbConfig XkbConfig?
---@field SetLibinputSetting LibinputSetting?
---@field Request Request?

---@alias Msg _Msg | "Quit"

---@alias FullscreenOrMaximized
---| "Neither"
---| "Fullscreen"
---| "Maximized"

--------------------------------------------------------------------------------------------

---@class __Request
--Windows
---@field GetWindowProps { window_id: WindowId }?
--Outputs
---@field GetOutputProps { output_name: string }?
--Tags
---@field GetTagProps { tag_id: TagId }?

---@alias _Request __Request | "GetWindows" | "GetOutputs" | "GetTags"
---@alias Request { request_id: integer, request: _Request }

---@class IncomingMsg
---@field CallCallback { callback_id: integer, args: Args? }?
---@field RequestResponse { request_id: integer, response: RequestResponse }?

---@class Args
---@field Spawn { stdout: string?, stderr: string?, exit_code: integer?, exit_msg: string? }?
---@field ConnectForAllOutputs { output_name: string }?

---@alias WindowId integer
---@alias TagId integer
---@alias RequestId integer
---@alias OutputName string

---@class RequestResponse
--Windows
---@field Window { window_id: WindowId|nil }?
---@field Windows { window_ids: WindowId[] }?
---@field WindowProps { size: integer[]?, loc: integer[]?, class: string?, title: string?, focused: boolean?, floating: boolean?, fullscreen_or_maximized: FullscreenOrMaximized? }?
--Outputs
---@field Output { output_name: OutputName? }?
---@field Outputs { output_names: OutputName[] }?
---@field OutputProps { make: string?, model: string?, loc: integer[]?, res: integer[]?, refresh_rate: integer?, physical_size: integer[]?, focused: boolean?, tag_ids: integer[]? }?
--Tags
---@field Tags { tag_ids: TagId[] }?
---@field TagProps { active: boolean?, name: string?, output_name: string? }?
