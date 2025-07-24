local cqueues = require("cqueues")
local monotime = cqueues.monotime
local ce = require("cqueues.errno")

---@class pinnacle.grpc.StreamExtension
local StreamExtension = {}
local extension_methods = {}

---Call h2_stream:get_headers, retrying up to `retries` time.
---
---@param timeout number
---@param retries integer
---@return http.headers|nil
---@return string
---@return number
function extension_methods:get_headers_with_retries(timeout, retries)
    local retry = retries or 1

    if retry < 1 then
        retry = 1
    end

    for i = 1, retry do
        local headers, err, errno = self:get_headers(timeout)

        if headers then
            return headers
        elseif errno == ce.ETIMEDOUT and i < retry then
            -- Sometime, the get_headers can lockup for no good reason.
            -- This allows us to proceed if that was the reason we timedout
            self.connection:step(0)
        else
            return nil, err, errno
        end
    end
end

---Extend a stream with new methods
---
---@param s http.h2_stream.stream
---@return http.h2_stream.stream
function StreamExtension.extend(s)
    for k, v in pairs(extension_methods) do
        s[k] = v
    end
    return s
end

StreamExtension.methods = extension_methods

return StreamExtension
