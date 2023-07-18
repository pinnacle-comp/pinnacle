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
---@field CloseWindow { client_id: integer? }
---@field ToggleFloating { client_id: integer? }
---@field SetWindowSize { window_id: integer, size: { w: integer, h: integer } }
---@field MoveWindowToTag { window_id: integer, tag_id: string }
---@field ToggleTagOnWindow { window_id: integer, tag_id: string }
--
---@field Spawn { command: string[], callback_id: integer? }
---@field Request Request
--Tags
---@field ToggleTag { output_name: string, tag_name: string }
---@field SwitchToTag { output_name: string, tag_name: string }
---@field AddTags { output_name: string, tags: string[] }
---@field RemoveTags { output_name: string, tags: string[] }
---@field SetLayout { output_name: string, tag_name: string, layout: Layout }
--Outputs
---@field ConnectForAllOutputs { callback_id: integer }

---@alias Msg _Msg | "Quit"

--------------------------------------------------------------------------------------------

---@class _Request
--Windows
---@field GetWindowByAppId { app_id: string }
---@field GetWindowByTitle { title: string }
--Outputs
---@field GetOutputByName { name: string }
---@field GetOutputsByModel { model: string }
---@field GetOutputsByRes { res: integer[] }
---@field GetTagsByOutput { output: string }
---@field GetTagActive { tag_id: integer }
---@field GetTagName { tag_id: integer }

---@alias Request _Request | "GetWindowByFocus" | "GetAllWindows" | "GetOutputByFocus"

---@class IncomingMsg
---@field CallCallback { callback_id: integer, args: Args }
---@field RequestResponse { response: RequestResponse }

---@class Args
---@field Spawn { stdout: string?, stderr: string?, exit_code: integer?, exit_msg: string? }
---@field ConnectForAllOutputs { output_name: string }

---@class RequestResponse
---@field Window { window: WindowProperties }
---@field GetAllWindows { windows: WindowProperties[] }
---@field Outputs { names: string[] }
---@field Tags { tags: TagProperties[] }
---@field TagActive { active: boolean }
---@field TagName { name: string }

---@class WindowProperties
---@field id integer
---@field app_id string?
---@field title string?
---@field size integer[] A two element int array, \[1\] = w, \[2\] = h
---@field location integer[] A two element int array, \[1\] = x, \[2\] = y
---@field floating boolean

---@class TagProperties
---@field id integer
