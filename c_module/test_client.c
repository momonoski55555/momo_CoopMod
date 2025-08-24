// test_dll.c - Test program to use the DLL
#include <stdio.h>
#include <windows.h>
#include "pipe_client_dll.h"

int main() {
    printf("=== Testing Pipe Client DLL ===\n");

    PipeResult result;
    const char* pipeName = "\\\\.\\pipe\\cruesader_pipe";
    const char* message = "Hello from DLL test program!";

    printf("Attempting to connect to pipe: %s\n", pipeName);

    int success = ConnectToPipe(pipeName, message, &result);

    if (success && result.success) {
        printf("✓ Connection successful!\n");
        printf("✓ Received from server: '%s'\n", result.received_message);
        printf("✓ Sent message: '%s'\n", message);
    }
    else {
        printf("✗ Connection failed!\n");
        printf("✗ Error: %s\n", result.error_message);
        printf("✗ Error Code: %lu\n", result.error_code);
    }

    printf("\nPress Enter to exit...");
    getchar();
    return 0;
}