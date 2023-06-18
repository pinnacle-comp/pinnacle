-- require("luarocks.loader")

local LOCAL_PATH = "/home/jason/projects/pinnacle/pinnacle_api_lua"

package.path = LOCAL_PATH .. "/lib/?.lua;" .. LOCAL_PATH .. "/lib/?/init.lua;" .. package.path
package.cpath = LOCAL_PATH .. "/lib/?.so;" .. package.cpath

local socket = require("posix.sys.socket")
local fcntl = require("posix.fcntl")
local msgpack = require("msgpack")

local SOCKET_PATH = "/tmp/pinnacle_socket"

local CONFIG_PATH = (os.getenv("XDG_CONFIG_HOME") or "~/.config") .. "/pinnacle/init.lua"

package.path = CONFIG_PATH .. ";" .. package.path

---@type integer
local socket_fd = assert(socket.socket(socket.AF_UNIX, socket.SOCK_STREAM, 0), "Failed to create socket")
print("created socket at fd " .. socket_fd)

assert(0 == socket.connect(socket_fd, {
    family = socket.AF_UNIX,
    path = SOCKET_PATH,
}), "Failed to connect to Pinnacle socket")

function SendMsg(data)
    local encoded = msgpack.encode(data)
    assert(encoded)
    local len = encoded:len()
    socket.send(socket_fd, string.pack("=I4", len))
    socket.send(socket_fd, encoded)
end

---@type function[]
CallbackTable = {}

assert(pcall(require, "pinnacle"), "config file not found")

---Read the specified number of bytes.
---@param socket_fd integer The socket file descriptor
---@param count integer The amount of bytes to read
---@return string|nil data
---@return string|nil err_msg
---@return integer|nil err_num
local function read_exact(socket_fd, count)
    local len_to_read = count
    local data = ""
    while len_to_read > 0 do
        local bytes, err_msg, errnum = socket.recv(socket_fd, len_to_read)

        if bytes == nil then
            -- TODO: handle errors
            print("bytes was nil")
            return bytes, err_msg, errnum
        end

        ---@type integer
        local recv_len = bytes:len()

        if recv_len == 0 then
            print("stream closed")
            break
        end

        len_to_read = len_to_read - recv_len
        assert(len_to_read >= 0, "Overread message boundary")

        data = data .. bytes
    end
    return data
end

-- TODO: set timeouts so that you actually make sure the msg is correct
while true do
    local msg_len_bytes, err_msg, err_num = read_exact(socket_fd, 4)
    assert(msg_len_bytes)

    ---@type integer
    local msg_len = string.unpack("=I4", msg_len_bytes)

    local msg_bytes, err_msg2, err_num2 = read_exact(socket_fd, msg_len)
    assert(msg_bytes)

    local tb = msgpack.decode(msg_bytes)

    if tb.CallCallback then
        CallbackTable[tb.CallCallback]()
    end
end
