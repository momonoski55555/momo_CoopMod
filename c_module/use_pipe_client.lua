local ffi = require("ffi")

-- Define the C structure and function signature
ffi.cdef[[
    typedef struct {
        int success;
        char received_message[1024]; // Must match BUFFER_SIZE from C code
        char error_message[256];
        unsigned long error_code; // DWORD is unsigned long on Windows
    } PipeResult;

    __declspec(dllimport) int ConnectToPipe(const char* pipeName, const char* messageToSend, PipeResult* result);
]]

-- Load the DLL
local pipe_client = ffi.load("pipe_client.dll")

-- Create a PipeResult structure
local result = ffi.new("PipeResult[1]") -- Create an array of 1 PipeResult

local pipe_name = "\\\\.\\pipe\\cruesader_pipe"
local message_to_send = "Hello from Lua client!"

print("Attempting to connect to pipe: " .. pipe_name)

-- Call the C function
local success_code = pipe_client.ConnectToPipe(pipe_name, message_to_send, result)

if success_code ~= 0 then -- ConnectToPipe returns 1 for success, 0 for failure directly
    print("✓ Connection successful!")
    print("✓ Received from server: '" .. ffi.string(result[0].received_message) .. "'")
    print("✓ Sent message: '" .. message_to_send .. "'")
else
    print("✗ Connection failed!")
    print("✗ Error: " .. ffi.string(result[0].error_message))
    print("✗ Error Code: " .. result[0].error_code)
end

print("\nScript finished.")