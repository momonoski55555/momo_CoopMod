use std::ffi::OsStr;
use std::io;
use std::os::windows::ffi::OsStrExt;
use std::ptr;
use std::thread;
use std::time::Duration;
use std::iter::once;
use std::error::Error; // Added for e.source().and_then(|e| e.downcast_ref::<PipeError>())

// These are the core imports for Windows API functions and types from the 'winapi' crate.
// They are correctly organized by module (`minwindef`, `fileapi`, `namedpipeapi`, etc.)
// to ensure a clean and readable dependency list.

// Basic types for defining function signatures and data structures.
use winapi::shared::minwindef::{DWORD, FALSE, LPVOID, BOOL as WINBOOL};
use winapi::shared::ntdef::HANDLE;

// Constants for handle management and invalid values.
use winapi::um::handleapi::INVALID_HANDLE_VALUE;

// File and handle-related functions.
use winapi::um::fileapi::{CreateFileW, ReadFile, WriteFile, OPEN_EXISTING};
use winapi::um::handleapi::CloseHandle;

// Error handling functions.
use winapi::um::errhandlingapi::GetLastError;

// Specific functions for named pipes.
use winapi::um::namedpipeapi::{CreateNamedPipeW, ConnectNamedPipe, DisconnectNamedPipe};

// Access rights and creation disposition flags.
// These are used by both server and client to define how the pipe is opened.
// The constants for file and pipe access are located in the winnt module.
use winapi::um::winnt::{GENERIC_READ, GENERIC_WRITE, FILE_SHARE_READ, FILE_SHARE_WRITE};
use winapi::um::winbase::{
    PIPE_ACCESS_DUPLEX, PIPE_TYPE_BYTE, PIPE_READMODE_BYTE,
    PIPE_WAIT, PIPE_UNLIMITED_INSTANCES
};
use winapi::shared::winerror::ERROR_BROKEN_PIPE; // Moved to shared::winerror


// Constants for our named pipe
const PIPE_NAME: &str = "\\\\.\\pipe\\cruesader_pipe";
const BUFFER_SIZE: usize = 1024;

// Helper function to convert a Rust string slice to a null-terminated wide character string
// This is necessary because many Windows API functions expect UTF-16 strings.
fn lp_w_str(s: &str) -> Vec<u16> {
    OsStr::new(s).encode_wide().chain(once(0)).collect()
}

// Custom error type to provide more context for failed Windows API calls.
#[derive(Debug)]
struct PipeError {
    message: String,
    error_code: u32,
}

impl std::fmt::Display for PipeError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{} (Error Code: {})", self.message, self.error_code)
    }
}

impl std::error::Error for PipeError {}

impl From<PipeError> for io::Error {
    fn from(err: PipeError) -> Self {
        io::Error::new(io::ErrorKind::Other, err)
    }
}

// Safe wrapper for GetLastError, which retrieves the calling thread's last-error code.
fn get_last_error() -> u32 {
    unsafe { GetLastError() }
}

// Server implementation compatible with C clients.
fn run_server() -> io::Result<()> {
    println!("[Rust Server] Creating named pipe: {}", PIPE_NAME);

    let name_wide = lp_w_str(PIPE_NAME);

    // Create the named pipe.
    // We use PIPE_ACCESS_DUPLEX for both reading and writing, and
    // PIPE_TYPE_BYTE to ensure compatibility with C clients.
    let pipe_handle: HANDLE = unsafe {
        CreateNamedPipeW(
            name_wide.as_ptr(),
            PIPE_ACCESS_DUPLEX,
            PIPE_TYPE_BYTE | PIPE_READMODE_BYTE | PIPE_WAIT,
            PIPE_UNLIMITED_INSTANCES,
            BUFFER_SIZE as DWORD,
            BUFFER_SIZE as DWORD,
            0,
            ptr::null_mut(),
        )
    };

    if pipe_handle == INVALID_HANDLE_VALUE {
        let error_code = get_last_error();
        return Err(PipeError {
            message: "Failed to create named pipe".to_string(),
            error_code,
        }.into());
    }

    println!("[Rust Server] Pipe created successfully. Waiting for C client connection...");

    // Wait for a client to connect to the pipe. This call blocks until a client connects.
    let connect_success: WINBOOL = unsafe {
        ConnectNamedPipe(pipe_handle, ptr::null_mut())
    };

    if connect_success == FALSE {
        let error_code = get_last_error();
        unsafe { CloseHandle(pipe_handle); }
        return Err(PipeError {
            message: "Failed to connect client".to_string(),
            error_code,
        }.into());
    }

    println!("[Rust Server] C client connected successfully.");

    // Send a message to the connected client.
    let server_message = "ibrahim is buetiful.\n";
    if let Err(e) = write_to_pipe(pipe_handle, server_message.as_bytes()) {
        unsafe {
            DisconnectNamedPipe(pipe_handle);
            CloseHandle(pipe_handle);
        }
        return Err(e);
    }
    println!("[Rust Server] Sent message to C client: '{}'", server_message.trim());

    // Read the response from the client.
    println!("[Rust Server] Waiting to read response from C client...");
    match read_from_pipe(pipe_handle) {
        Ok(response) => {
            let received_message = String::from_utf8_lossy(&response);
            println!("[Rust Server] Received from C client: '{}'", received_message.trim());
        }
        Err(e) => {
            // Check for a broken pipe error, which indicates the client has disconnected.
            if let Some(pipe_err) = e.source().and_then(|e| e.downcast_ref::<PipeError>()) {
                if pipe_err.error_code == ERROR_BROKEN_PIPE {
                    println!("[Rust Server] C client disconnected (broken pipe).");
                } else {
                    eprintln!("[Rust Server] Failed to read from pipe: {}", e);
                }
            } else {
                eprintln!("[Rust Server] Failed to read from pipe: {}", e);
            }
        }
    }

    // Cleanup resources by disconnecting the pipe and closing the handle.
    unsafe {
        DisconnectNamedPipe(pipe_handle);
        CloseHandle(pipe_handle);
    }
    println!("[Rust Server] Connection closed and resources cleaned up.");

    Ok(())
}

// Helper function for writing data to the pipe with robust error handling.
fn write_to_pipe(pipe_handle: HANDLE, data: &[u8]) -> io::Result<()> {
    let mut bytes_written: DWORD = 0;

    let write_success: WINBOOL = unsafe {
        WriteFile(
            pipe_handle,
            data.as_ptr() as LPVOID,
            data.len() as DWORD,
            &mut bytes_written,
            ptr::null_mut(),
        )
    };

    if write_success == FALSE || bytes_written != data.len() as DWORD {
        let error_code = get_last_error();
        return Err(PipeError {
            message: "Failed to write to pipe".to_string(),
            error_code,
        }.into());
    }

    Ok(())
}

// Helper function for reading data from the pipe into a buffer.
fn read_from_pipe(pipe_handle: HANDLE) -> io::Result<Vec<u8>> {
    let mut buffer: [u8; BUFFER_SIZE] = [0; BUFFER_SIZE];
    let mut bytes_read: DWORD = 0;

    let read_success: WINBOOL = unsafe {
        ReadFile(
            pipe_handle,
            buffer.as_mut_ptr() as LPVOID,
            BUFFER_SIZE as DWORD,
            &mut bytes_read,
            ptr::null_mut(),
        )
    };

    if read_success == FALSE {
        let error_code = get_last_error();
        return Err(PipeError {
            message: "Failed to read from pipe".to_string(),
            error_code,
        }.into());
    }

    Ok(buffer[..bytes_read as usize].to_vec())
}

// The main function, which creates and runs the server in a continuous loop.
fn main() -> io::Result<()> {
    println!("=== Rust Named Pipe Server ===");
    println!("This server is compatible with C clients.");
    println!("Compile the C client and run it after starting this server.");
    println!();

    // The server runs in an infinite loop to handle multiple client connections sequentially.
    loop {
        println!("Starting new server instance...");
        match run_server() {
            Ok(()) => {
                println!("Server completed successfully.");
            }
            Err(e) => {
                eprintln!("Server error: {}", e);
            }
        }
        
        println!();
        println!("Waiting 2 seconds before starting next server instance...");
        println!("Press Ctrl+C to exit.");
        thread::sleep(Duration::from_secs(2));
    }
}
