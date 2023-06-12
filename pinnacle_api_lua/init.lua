-- require("luarocks.loader")

package.cpath = "./lib/?.so;" .. package.cpath

local socket = require("posix.sys.socket")
local unistd = require("posix.unistd")
local ffi = require("cffi")

local SOCKET_PATH = "/tmp/pinnacle_socket"

local sockaddr = {
    family = socket.AF_UNIX,
    path = SOCKET_PATH,
}

local socket_fd = assert(socket.socket(socket.AF_UNIX, socket.SOCK_STREAM, 0))
print("created socket at fd " .. socket_fd)

if 0 ~= socket.connect(socket_fd, sockaddr) then
    assert(false)
end

ffi.cdef([[
typedef struct Message { uint32_t number; uint8_t number2; } message;
]])

local type = ffi.typeof("message")

local struct = ffi.new(type, {
    number = 12,
    number2 = 254,
})
local size = ffi.sizeof("message")
local str = ffi.string(struct, size)

socket.send(socket_fd, str)

-- unistd.close(socket_fd)
