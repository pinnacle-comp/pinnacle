-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
--
-- SPDX-License-Identifier: MPL-2.0

local socket = require("posix.sys.socket")
local msgpack = require("msgpack")

local SOCKET_PATH = "/tmp/pinnacle_socket"

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

---@class Pinnacle
---The main Pinnacle table, where all of the config options come from.
---
---While you *can* import the fields directly, all config must be in the `setup` function, so you might as well just use the provided table. The ability to directly `require` fields may be dropped in the future.
local pinnacle = {
    ---Key and mouse binds
    input = require("input"),
    ---Window management
    client = require("client"),
    ---Process spawning
    process = require("process"),
    ---Tag management
    tag = require("tag"),
}

---Quit Pinnacle.
function pinnacle.quit()
    SendMsg("Quit")
end

---Configure Pinnacle. You should put mostly eveything into the config_func to avoid invalid state.
---The function takes one argument: the Pinnacle table, which is how you'll access all of the available config options.
---@param config_func fun(pinnacle: Pinnacle)
function pinnacle.setup(config_func)
    ---@type integer
    local socket_fd = assert(socket.socket(socket.AF_UNIX, socket.SOCK_STREAM, 0), "Failed to create socket")
    print("created socket at fd " .. socket_fd)

    assert(0 == socket.connect(socket_fd, {
        family = socket.AF_UNIX,
        path = SOCKET_PATH,
    }), "Failed to connect to Pinnacle socket")

    ---@type fun(args: table?)[]
    CallbackTable = {}

    ---@param data Msg
    function SendMsg(data)
        local encoded = msgpack.encode(data)
        assert(encoded)
        local len = encoded:len()
        socket.send(socket_fd, string.pack("=I4", len))
        socket.send(socket_fd, encoded)
    end

    ---@param data Request
    function SendRequest(data)
        SendMsg({
            Request = data,
        })
    end

    function ReadMsg()
        local msg_len_bytes, err_msg, err_num = read_exact(socket_fd, 4)
        assert(msg_len_bytes)

        ---@type integer
        local msg_len = string.unpack("=I4", msg_len_bytes)

        local msg_bytes, err_msg2, err_num2 = read_exact(socket_fd, msg_len)
        assert(msg_bytes)

        ---@type IncomingMsg
        local tb = msgpack.decode(msg_bytes)
        -- print(msg_bytes)

        return tb
    end

    Requests = {
        id = 1,
    }
    function Requests:next()
        local id = self.id
        self.id = self.id + 1
        return id
    end

    config_func(pinnacle)

    while true do
        local tb = ReadMsg()

        if tb.CallCallback and tb.CallCallback.callback_id then
            if tb.CallCallback.args then -- TODO: can just inline
                CallbackTable[tb.CallCallback.callback_id](tb.CallCallback.args)
            else
                CallbackTable[tb.CallCallback.callback_id](nil)
            end
        end

        -- if tb.RequestResponse then
        --     local req_id = tb.RequestResponse.request_id
        --     Requests[req_id] = tb.RequestResponse.response
        -- end
    end
end

return pinnacle
