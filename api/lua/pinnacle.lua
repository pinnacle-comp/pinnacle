-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.
--
-- SPDX-License-Identifier: MPL-2.0

local socket = require("posix.sys.socket")
local msgpack = require("msgpack")

local SOCKET_PATH = "/tmp/pinnacle_socket"

---From https://gist.github.com/stuby/5445834#file-rprint-lua
---rPrint(struct, [limit], [indent])   Recursively print arbitrary data.
---	Set limit (default 100) to stanch infinite loops.
---	Indents tables as [KEY] VALUE, nested tables as [KEY] [KEY]...[KEY] VALUE
---	Set indent ("") to prefix each line:    Mytable [KEY] [KEY]...[KEY] VALUE
---@param s table The table
---@param l integer? Recursion limit
---@param i string? The indent string
---@return integer l The remaining depth limit
function RPrint(s, l, i) -- recursive Print (structure, limit, indent)
    l = l or 100
    i = i or "" -- default item limit, indent string
    if l < 1 then
        print("ERROR: Item limit reached.")
        return l - 1
    end
    local ts = type(s)
    if ts ~= "table" then
        print(i, ts, s)
        return l - 1
    end
    print(i, ts) -- print "table"
    for k, v in pairs(s) do -- print "[KEY] VALUE"
        l = RPrint(v, l, i .. "\t[" .. tostring(k) .. "]")
        if l < 0 then
            break
        end
    end
    return l
end

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
        -- print("need to read " .. tostring(len_to_read) .. " bytes")
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
    window = require("window"),
    ---Process spawning
    process = require("process"),
    ---Tag management
    tag = require("tag"),
    ---Output management
    output = require("output"),
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

    ---This is an internal global function used to send serialized messages to the Pinnacle server.
    ---@param data Msg
    function SendMsg(data)
        -- RPrint(data)
        local encoded = msgpack.encode(data)
        assert(encoded)
        -- print(encoded)
        local len = encoded:len()
        socket.send(socket_fd, string.pack("=I4", len))
        socket.send(socket_fd, encoded)
    end

    local request_id = 1
    ---Get the next request id.
    ---@return integer
    local function next_request_id()
        local ret = request_id
        request_id = request_id + 1
        return ret
    end

    ---@type table<integer, IncomingMsg>
    local unread_req_msgs = {}
    ---@type table<integer, IncomingMsg>
    local unread_cb_msgs = {}

    ---This is an internal global function used to send requests to the Pinnacle server for information.
    ---@param data _Request
    ---@return IncomingMsg
    function Request(data)
        local req_id = next_request_id()
        SendMsg({
            Request = {
                request_id = req_id,
                request = data,
            },
        })
        return ReadMsg(req_id)
    end

    ---This is an internal global function used to read messages sent from the server.
    ---These are used to call user-defined functions and provide requested information.
    ---@return IncomingMsg
    ---@param req_id integer? A request id if you're looking for that specific message.
    function ReadMsg(req_id)
        while true do
            if req_id then
                if unread_req_msgs[req_id] then
                    local msg = unread_req_msgs[req_id]
                    unread_req_msgs[req_id] = nil -- INFO: is this a reference?
                    return msg
                end
            end

            local msg_len_bytes, err_msg, err_num = read_exact(socket_fd, 4)
            assert(msg_len_bytes)

            -- TODO: break here if error in read_exact

            ---@type integer
            local msg_len = string.unpack("=I4", msg_len_bytes)
            -- print(msg_len)

            local msg_bytes, err_msg2, err_num2 = read_exact(socket_fd, msg_len)
            assert(msg_bytes)
            -- print(msg_bytes)

            ---@type IncomingMsg
            local inc_msg = msgpack.decode(msg_bytes)
            -- print(msg_bytes)

            if req_id then
                if inc_msg.CallCallback then
                    unread_cb_msgs[inc_msg.CallCallback.callback_id] = inc_msg
                elseif inc_msg.RequestResponse.request_id ~= req_id then
                    unread_req_msgs[inc_msg.RequestResponse.request_id] = inc_msg
                else
                    return inc_msg
                end
            else
                return inc_msg
            end
        end
    end

    config_func(pinnacle)

    while true do
        for cb_id, inc_msg in pairs(unread_cb_msgs) do
            CallbackTable[inc_msg.CallCallback.callback_id](inc_msg.CallCallback.args)
            unread_cb_msgs[cb_id] = nil -- INFO: does this shift the table and frick everything up?
        end

        local inc_msg = ReadMsg()

        assert(inc_msg.CallCallback) -- INFO: is this gucci or no

        if inc_msg.CallCallback and inc_msg.CallCallback.callback_id then
            if inc_msg.CallCallback.args then -- TODO: can just inline
                CallbackTable[inc_msg.CallCallback.callback_id](inc_msg.CallCallback.args)
            else
                CallbackTable[inc_msg.CallCallback.callback_id](nil)
            end
        end
    end
end

return pinnacle
