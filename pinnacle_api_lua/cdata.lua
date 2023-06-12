local ffi = require("ffi")

local cdata = {}

-- http://www.catb.org/esr/structure-packing/
function cdata:new_struct(name, struct)
    ffi.cdef(struct)

    self.structs = self.structs or {}
    self.structs[name] = ffi.typeof(name)

    self.pointers = self.pointers or {}
    self.pointers[name] = ffi.typeof(name .. "*")
end

function cdata:set_struct(name, data)
    return ffi.new(self.structs[name], data)
end

function cdata:encode(data)
    print("cast")
    local cast = ffi.cast("const char*", data)
    print("sizeof")
    local size = ffi.sizeof(data)
    print("string")
    return ffi.string(cast, size)
end

function cdata:decode(name, data)
    return ffi.cast(self.pointers[name], data)[0]
end

return cdata
