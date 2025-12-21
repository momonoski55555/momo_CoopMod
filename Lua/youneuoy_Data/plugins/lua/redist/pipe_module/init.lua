-- redist/pipe_module/init.lua
-- This module encapsulates named pipe client functionality.

local ffi = require("ffi")

-- Define the C structure and function signature (must match C DLL header)
ffi.cdef[[
    typedef struct {
        int success;
        char received_message[1024]; // Must match BUFFER_SIZE from C code
        char error_message[256];
        unsigned long error_code; // DWORD is unsigned long on Windows
    } PipeResult;

    __declspec(dllimport) int CallPipeServer(const char* pipeName, const char* messageToSend, PipeResult* result);
]]

local M = {} -- Our module table

-- Construct the absolute path to core.dll relative to this script.
local SCRIPT_PATH = debug.getinfo(1, "S").source:sub(2) -- Get absolute path of this script (remove leading '@')
local SCRIPT_DIR = SCRIPT_PATH:match("(.*/)") -- Extract directory path, including trailing slash
local DLL_FULL_PATH = SCRIPT_DIR .. "core.dll"

print("Attempting to load DLL from: " .. DLL_FULL_PATH) -- Debugging output

-- Load the DLL globally once using its absolute path.
local pipe_client_dll_loader = ffi.load(DLL_FULL_PATH)

if not pipe_client_dll_loader then
    error("Failed to load DLL from '" .. DLL_FULL_PATH .. "'. Ensure 'core.dll' exists at this location.")
end
print("✓ Successfully loaded DLL from: " .. DLL_FULL_PATH) -- Debugging output

-- Helper function for pipe connection
function M.connect(pipe_name, message_to_send)
    local result = ffi.new("PipeResult[1]")
    
    local success_code = pipe_client_dll_loader.CallPipeServer(pipe_name, message_to_send, result)
    
    local output = {
        success = (success_code ~= 0),
        received_message = ffi.string(result[0].received_message),
        error_message = ffi.string(result[0].error_message),
        error_code = result[0].error_code
    }

    if not output.success then
        -- Map common error codes to user-friendly messages
        local error_descriptions = {
            [2] = "File Not Found (pipe server not running OR pipe name mismatch)",
            [5] = "Access Denied (check permissions)",
            [231] = "Pipe Busy (server not accepting connections)",
            [109] = "Broken Pipe (connection terminated)",
            [53] = "Network Path Not Found (not applicable for local pipes)"
        }
        local description = error_descriptions[output.error_code] or "Unknown Error"
        output.full_error = string.format("Error %d: %s - %s", output.error_code, description, output.error_message)
    end

    return output
end

return M