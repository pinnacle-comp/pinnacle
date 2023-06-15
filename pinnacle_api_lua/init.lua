-- require("luarocks.loader")

package.path = "./lib/?.lua;./lib/?/init.lua;" .. package.path
package.cpath = "./lib/?.so;" .. package.cpath

local socket = require("posix.sys.socket")
local msgpack = require("lib.msgpack")

local SOCKET_PATH = "/tmp/pinnacle_socket"

local CONFIG_PATH = os.getenv("XDG_CONFIG_HOME") .. "/pinnacle/init.lua"

package.path = CONFIG_PATH .. ";" .. package.path

local sockaddr = {
    family = socket.AF_UNIX,
    path = SOCKET_PATH,
}

local socket_fd = assert(socket.socket(socket.AF_UNIX, socket.SOCK_STREAM, 0), "Failed to create socket")
print("created socket at fd " .. socket_fd)

assert(0 == socket.connect(socket_fd, sockaddr), "Failed to connect to Pinnacle socket")

function SendMsg(data)
    socket.send(socket_fd, msgpack.encode(data))
end

---@type function[]
CallbackTable = {}

assert(pcall(require, "pinnacle"), "config file not found")

-- local str = msgpack.encode({
--     SetMousebind = { button = 6 },
-- })
-- local str = msgpack.encode({
--     SetKeybind = {
--         key = "This is a key",
--         modifiers = { "ctrl", "boogers", "numpty" },
--     },
-- })
-- print(str)
--
-- socket.send(socket_fd, str)

-- unistd.close(socket_fd)

-- local keys = require("keys")
--
-- local input = require("input")
-- input.keybind({ "Shift", "Ctrl" }, keys.c, "CloseWindow")
while true do
end
