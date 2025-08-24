// pipe_client.c
#include <windows.h>
#include <stdio.h>
#include <string.h>

#define BUFFER_SIZE 1024
#define PIPE_NAME "\\\\.\\pipe\\cruesader_pipe"
#define MAX_RETRY_ATTEMPTS 10
#define RETRY_DELAY_MS 200

// Export macro for DLL functions
#define BUILDING_DLL
#ifdef BUILDING_DLL
#define DLL_EXPORT __declspec(dllexport)
#else
#define DLL_EXPORT __declspec(dllimport)
#endif

// Structure to hold pipe communication results
typedef struct {
    int success;
    char received_message[BUFFER_SIZE];
    char error_message[256];
    DWORD error_code;
} PipeResult;

// DLL entry point
BOOL WINAPI DllMain(HINSTANCE hinstDLL, DWORD fdwReason, LPVOID lpvReserved) {
    switch (fdwReason) {
    case DLL_PROCESS_ATTACH:
        // DLL is being loaded
        break;
    case DLL_PROCESS_DETACH:
        // DLL is being unloaded
        break;
    case DLL_THREAD_DETACH:
        // A thread is being destroyed
        break;
    }
    return TRUE;
}

// Main pipe client function - exported from DLL
DLL_EXPORT int ConnectToPipe(const char* pipeName, const char* messageToSend, PipeResult* result) {
    if (!pipeName) pipeName = PIPE_NAME;
    if (!result) return 0;

    // Initialize result structure
    memset(result, 0, sizeof(PipeResult));

    HANDLE hPipe = INVALID_HANDLE_VALUE;
    int attempt;

    // Retry connection with delay
    for (attempt = 1; attempt <= MAX_RETRY_ATTEMPTS; attempt++) {
        hPipe = CreateFileA(
            pipeName,
            GENERIC_READ | GENERIC_WRITE,
            0,
            NULL,
            OPEN_EXISTING,
            0,
            NULL);

        if (hPipe != INVALID_HANDLE_VALUE) {
            break;
        }

        DWORD error = GetLastError();
        if (attempt == MAX_RETRY_ATTEMPTS) {
            result->error_code = error;
            snprintf(result->error_message, sizeof(result->error_message),
                "Failed to connect after %d attempts. Error: %lu", MAX_RETRY_ATTEMPTS, error);
            return 0;
        }

        Sleep(RETRY_DELAY_MS * attempt);
    }

    // Read message from server
    char readBuffer[BUFFER_SIZE];
    DWORD bytesRead;

    BOOL readSuccess = ReadFile(hPipe, readBuffer, sizeof(readBuffer) - 1, &bytesRead, NULL);

    if (!readSuccess || bytesRead == 0) {
        result->error_code = GetLastError();
        snprintf(result->error_message, sizeof(result->error_message),
            "Failed to read from pipe. Error: %lu", result->error_code);
        CloseHandle(hPipe);
        return 0;
    }

    readBuffer[bytesRead] = '\0';
    snprintf(result->received_message, sizeof(result->received_message), "%s", readBuffer);

    // Send response to server
    const char* response = messageToSend ? messageToSend : "Hello from DLL client!";
    DWORD bytesWritten;

    BOOL writeSuccess = WriteFile(hPipe, response, (DWORD)strlen(response), &bytesWritten, NULL);

    if (!writeSuccess || bytesWritten != strlen(response)) {
        result->error_code = GetLastError();
        snprintf(result->error_message, sizeof(result->error_message),
            "Failed to write to pipe. Error: %lu", result->error_code);
        CloseHandle(hPipe);
        return 0;
    }

    CloseHandle(hPipe);
    result->success = 1;
    return 1;
}
