// pipe_client_dll.h
#ifndef PIPE_CLIENT_DLL_H
#define PIPE_CLIENT_DLL_H

#include <windows.h>

#ifdef __cplusplus
extern "C" {
#endif

    // Export macro
#ifdef BUILDING_DLL
#define DLL_EXPORT __declspec(dllexport)
#else
#define DLL_EXPORT __declspec(dllimport)
#endif

#define BUFFER_SIZE 1024

// Structure to hold pipe communication results
    typedef struct {
        int success;
        char received_message[BUFFER_SIZE];
        char error_message[256];
        DWORD error_code;
    } PipeResult;

    // Function declarations
    DLL_EXPORT int ConnectToPipe(const char* pipeName, const char* messageToSend, PipeResult* result);

#ifdef __cplusplus
}
#endif

#endif // PIPE_CLIENT_DLL_H