-- SPDX-License-Identifier: Unlicense

--[[----------------------------------------------------------------------------

	MessagePack encoder / decoder written in pure Lua 5.3 / Lua 5.4
	written by Sebastian Steinhauer <s.steinhauer@yahoo.de>

	This is free and unencumbered software released into the public domain.

	Anyone is free to copy, modify, publish, use, compile, sell, or
	distribute this software, either in source code form or as a compiled
	binary, for any purpose, commercial or non-commercial, and by any
	means.

	In jurisdictions that recognize copyright laws, the author or authors
	of this software dedicate any and all copyright interest in the
	software to the public domain. We make this dedication for the benefit
	of the public at large and to the detriment of our heirs and
	successors. We intend this dedication to be an overt act of
	relinquishment in perpetuity of all present and future rights to this
	software under copyright law.

	THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND,
	EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF
	MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT.
	IN NO EVENT SHALL THE AUTHORS BE LIABLE FOR ANY CLAIM, DAMAGES OR
	OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE,
	ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR
	OTHER DEALINGS IN THE SOFTWARE.

	For more information, please refer to <http://unlicense.org/>

--]]
----------------------------------------------------------------------------
local pack, unpack = string.pack, string.unpack
local mtype, utf8len = math.type, utf8.len
local tconcat, tunpack = table.concat, table.unpack
local ssub = string.sub
local type, pcall, pairs, select = type, pcall, pairs, select

--[[----------------------------------------------------------------------------

		Encoder

--]]
----------------------------------------------------------------------------
local encode_value -- forward declaration

local function is_an_array(value)
    local expected = 1
    for k in pairs(value) do
        if k ~= expected then
            return false
        end
        expected = expected + 1
    end
    return true
end

local encoder_functions = {
    ["nil"] = function()
        return pack("B", 0xc0)
    end,
    ["boolean"] = function(value)
        if value then
            return pack("B", 0xc3)
        else
            return pack("B", 0xc2)
        end
    end,
    ["number"] = function(value)
        if mtype(value) == "integer" then
            if value >= 0 then
                if value < 128 then
                    return pack("B", value)
                elseif value <= 0xff then
                    return pack("BB", 0xcc, value)
                elseif value <= 0xffff then
                    return pack(">BI2", 0xcd, value)
                elseif value <= 0xffffffff then
                    return pack(">BI4", 0xce, value)
                else
                    return pack(">BI8", 0xcf, value)
                end
            else
                if value >= -32 then
                    return pack("B", 0xe0 + (value + 32))
                elseif value >= -128 then
                    return pack("Bb", 0xd0, value)
                elseif value >= -32768 then
                    return pack(">Bi2", 0xd1, value)
                elseif value >= -2147483648 then
                    return pack(">Bi4", 0xd2, value)
                else
                    return pack(">Bi8", 0xd3, value)
                end
            end
        else
            local test = unpack("f", pack("f", value))
            if test == value then -- check if we can use float
                return pack(">Bf", 0xca, value)
            else
                return pack(">Bd", 0xcb, value)
            end
        end
    end,
    ["string"] = function(value)
        local len = #value
        if utf8len(value) then -- check if it is a real utf8 string or just byte junk
            if value == "nil" then -- TODO: maybe just check for nil in api functions
                return pack("B", 0xc0)
            elseif len < 32 then
                return pack("B", 0xa0 + len) .. value
            elseif len < 256 then
                return pack(">Bs1", 0xd9, value)
            elseif len < 65536 then
                return pack(">Bs2", 0xda, value)
            else
                return pack(">Bs4", 0xdb, value)
            end
        else -- encode it as byte-junk :)
            if len < 256 then
                return pack(">Bs1", 0xc4, value)
            elseif len < 65536 then
                return pack(">Bs2", 0xc5, value)
            else
                return pack(">Bs4", 0xc6, value)
            end
        end
    end,
    ["table"] = function(value)
        if is_an_array(value) then -- it seems to be a proper Lua array
            local elements = {}
            for i, v in pairs(value) do
                elements[i] = encode_value(v)
            end

            local length = #elements
            if length < 16 then
                return pack(">B", 0x90 + length) .. tconcat(elements)
            elseif length < 65536 then
                return pack(">BI2", 0xdc, length) .. tconcat(elements)
            else
                return pack(">BI4", 0xdd, length) .. tconcat(elements)
            end
        else -- encode as a map
            local elements = {}
            for k, v in pairs(value) do
                elements[#elements + 1] = encode_value(k)
                elements[#elements + 1] = encode_value(v)
            end

            local length = #elements // 2
            if length < 16 then
                return pack(">B", 0x80 + length) .. tconcat(elements)
            elseif length < 65536 then
                return pack(">BI2", 0xde, length) .. tconcat(elements)
            else
                return pack(">BI4", 0xdf, length) .. tconcat(elements)
            end
        end
    end,
}

encode_value = function(value)
    return encoder_functions[type(value)](value)
end

local function encode(...)
    local data = {}
    for i = 1, select("#", ...) do
        data[#data + 1] = encode_value(select(i, ...))
    end
    return tconcat(data)
end

--[[----------------------------------------------------------------------------

		Decoder

--]]
----------------------------------------------------------------------------
local decode_value -- forward declaration

local function decode_array(data, position, length)
    local elements, value = {}
    for i = 1, length do
        value, position = decode_value(data, position)
        elements[i] = value
    end
    return elements, position
end

local function decode_map(data, position, length)
    local elements, key, value = {}
    for i = 1, length do
        key, position = decode_value(data, position)
        value, position = decode_value(data, position)
        elements[key] = value
    end
    return elements, position
end

local decoder_functions = {
    [0xc0] = function(data, position)
        return nil, position
    end,
    [0xc2] = function(data, position)
        return false, position
    end,
    [0xc3] = function(data, position)
        return true, position
    end,
    [0xc4] = function(data, position)
        return unpack(">s1", data, position)
    end,
    [0xc5] = function(data, position)
        return unpack(">s2", data, position)
    end,
    [0xc6] = function(data, position)
        return unpack(">s4", data, position)
    end,
    [0xca] = function(data, position)
        return unpack(">f", data, position)
    end,
    [0xcb] = function(data, position)
        return unpack(">d", data, position)
    end,
    [0xcc] = function(data, position)
        return unpack(">B", data, position)
    end,
    [0xcd] = function(data, position)
        return unpack(">I2", data, position)
    end,
    [0xce] = function(data, position)
        return unpack(">I4", data, position)
    end,
    [0xcf] = function(data, position)
        return unpack(">I8", data, position)
    end,
    [0xd0] = function(data, position)
        return unpack(">b", data, position)
    end,
    [0xd1] = function(data, position)
        return unpack(">i2", data, position)
    end,
    [0xd2] = function(data, position)
        return unpack(">i4", data, position)
    end,
    [0xd3] = function(data, position)
        return unpack(">i8", data, position)
    end,
    [0xd9] = function(data, position)
        return unpack(">s1", data, position)
    end,
    [0xda] = function(data, position)
        return unpack(">s2", data, position)
    end,
    [0xdb] = function(data, position)
        return unpack(">s4", data, position)
    end,
    [0xdc] = function(data, position)
        local length
        length, position = unpack(">I2", data, position)
        return decode_array(data, position, length)
    end,
    [0xdd] = function(data, position)
        local length
        length, position = unpack(">I4", data, position)
        return decode_array(data, position, length)
    end,
    [0xde] = function(data, position)
        local length
        length, position = unpack(">I2", data, position)
        return decode_map(data, position, length)
    end,
    [0xdf] = function(data, position)
        local length
        length, position = unpack(">I4", data, position)
        return decode_map(data, position, length)
    end,
}

-- add fix-array, fix-map, fix-string, fix-int stuff
for i = 0x00, 0x7f do
    decoder_functions[i] = function(data, position)
        return i, position
    end
end
for i = 0x80, 0x8f do
    decoder_functions[i] = function(data, position)
        return decode_map(data, position, i - 0x80)
    end
end
for i = 0x90, 0x9f do
    decoder_functions[i] = function(data, position)
        return decode_array(data, position, i - 0x90)
    end
end
for i = 0xa0, 0xbf do
    decoder_functions[i] = function(data, position)
        local length = i - 0xa0
        return ssub(data, position, position + length - 1), position + length
    end
end
for i = 0xe0, 0xff do
    decoder_functions[i] = function(data, position)
        return -32 + (i - 0xe0), position
    end
end

decode_value = function(data, position)
    local byte, value
    byte, position = unpack("B", data, position)
    value, position = decoder_functions[byte](data, position)
    return value, position
end

--[[----------------------------------------------------------------------------

		Interface

--]]
----------------------------------------------------------------------------
return {
    _AUTHOR = "Sebastian Steinhauer <s.steinhauer@yahoo.de>",
    _VERSION = "0.6.1",

    -- primary encode function
    encode = function(...)
        local data, ok = {}
        for i = 1, select("#", ...) do
            ok, data[i] = pcall(encode_value, select(i, ...))
            if not ok then
                return nil, "cannot encode MessagePack"
            end
        end
        return tconcat(data)
    end,

    -- encode just one value
    encode_one = function(value)
        local ok, data = pcall(encode_value, value)
        if ok then
            return data
        else
            return nil, "cannot encode MessagePack"
        end
    end,

    -- Decode a string of MessagePack data into objects.
    --
    ---@param data string The MessagePack packet
    ---@param position any
    ---@return any ... The decoded values unpacked. If decoding failed, nil
    ---@return string | nil error The string "cannot decode MessagePack" if decoding failed
    decode = function(data, position)
        local values, value, ok = {}
        position = position or 1
        while position <= #data do
            ok, value, position = pcall(decode_value, data, position)
            if ok then
                values[#values + 1] = value
            else
                return nil, "cannot decode MessagePack"
            end
        end
        return tunpack(values)
    end,

    -- decode just one value
    decode_one = function(data, position)
        local value, ok
        ok, value, position = pcall(decode_value, data, position or 1)
        if ok then
            return value, position
        else
            return nil, "cannot decode MessagePack"
        end
    end,
}

--[[----------------------------------------------------------------------------
--]]
----------------------------------------------------------------------------
