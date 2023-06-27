---@meta _

---@class _Msg
---@field SetKeybind { key: Keys, modifiers: Modifiers[], callback_id: integer }
---@field SetMousebind { button: integer }
---@field CloseWindow { client_id: integer? }
---@field ToggleFloating { client_id: integer? }
---@field SetWindowSize { window_id: integer, size: { w: integer, h: integer } }
---@field Spawn { command: string[], callback_id: integer? }
---@field Request Request

---@alias Msg _Msg | "Quit"

---@class Request
---@field GetWindowByFocus { id: integer }
---@field GetAllWindows { id: integer }

---@class IncomingMsg
---@field CallCallback { callback_id: integer, args: Args }
---@field RequestResponse { request_id: integer, response: RequestResponse }

---@class Args
---@field Spawn { stdout: string?, stderr: string?, exit_code: integer?, exit_msg: string? }

---@class RequestResponse
---@field Window { window: WindowProperties }
---@field GetAllWindows { windows: WindowProperties[] }

---@class WindowProperties
---@field id integer
---@field app_id string?
---@field title string?
---@field size integer[] A two element int array, \[1\] = w, \[2\] = h
---@field location integer[] A two element int array, \[1\] = x, \[2\] = y
---@field floating boolean
