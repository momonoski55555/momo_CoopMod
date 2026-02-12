local ffi = require("ffi")

ffi.cdef[[
    typedef struct {
        int success;
        char received_message[1024];
        char error_message[256];
        unsigned long error_code;
    } PipeResult;

    int CallPipeServer(const char* pipeName, const char* messageToSend, PipeResult* result);
]]

local M = {}

-- Extract directory path of the current script to find core.dll
local script_path = debug.getinfo(1, "S").source:sub(2)
local script_dir = script_path:match("(.*/)")
local dll_path = script_dir .. "core.dll"

local core = ffi.load(dll_path)
if not core then
    error("Could not load core.dll from " .. dll_path)
end

local ERROR_MAP = {
    [2] = "Pipe server not found",
    [5] = "Access denied",
    [231] = "Pipe busy",
    [109] = "Broken pipe",
}

function M.connect(pipe_name, message)
    local result = ffi.new("PipeResult[1]")
    local status = core.CallPipeServer(pipe_name, message, result)
    
    local res = result[0]
    local output = {
        success = (status ~= 0),
        message = ffi.string(res.received_message),
        error = ffi.string(res.error_message),
        code = tonumber(res.error_code)
    }

    if not output.success then
        local desc = ERROR_MAP[output.code] or "Unknown error"
        output.full_error = string.format("[%d] %s: %s", output.code, desc, output.error)
    end

    return output
end

return M
