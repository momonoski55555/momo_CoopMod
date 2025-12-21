#include <windows.h>
#include <stdio.h>

#define PIPE_NAME "\\\\.\\pipe\\coop_pipe"

typedef struct {
    DWORD win_error;
    char data[1024];
} PipeResponse;

#define DLL_EXPORT __declspec(dllexport)

BOOL WINAPI DllMain(HINSTANCE hinst, DWORD reason, LPVOID reserved) {
    return TRUE; 
}

DLL_EXPORT BOOL CallPipeServer(const char* send_msg, PipeResponse* out) {
    if (!out) return FALSE;
    
    HANDLE pipe = INVALID_HANDLE_VALUE;
    DWORD read, written;
    int retry = 0;

    while (retry < 5) {
        pipe = CreateFileA(PIPE_NAME, GENERIC_READ | GENERIC_WRITE, 0, NULL, OPEN_EXISTING, 0, NULL);
        if (pipe != INVALID_HANDLE_VALUE) break;

        if (GetLastError() != ERROR_PIPE_BUSY) {
            out->win_error = GetLastError();
            return FALSE;
        }

        WaitNamedPipeA(PIPE_NAME, 1000);
        retry++;
    }

    if (pipe == INVALID_HANDLE_VALUE) return FALSE;

    if (!ReadFile(pipe, out->data, sizeof(out->data) - 1, &read, NULL)) {
        out->win_error = GetLastError();
        CloseHandle(pipe);
        return FALSE;
    }
    out->data[read] = '\0';

    // Send response
    const char* msg = send_msg ? send_msg : "Client ACK";
    if (!WriteFile(pipe, msg, (DWORD)strlen(msg), &written, NULL)) {
        out->win_error = GetLastError();
    }

    CloseHandle(pipe);
    return (out->win_error == 0);
}