// diversify the services logic to not be in one script

// when turn is done rename it to the turn number then upload the save to a cloud service
// when its uploaded notify the server then download it 
// when downloaded load in saves save then continue





use std::ffi::OsStr;
use std::io;
use std::os::windows::ffi::OsStrExt;
use std::ptr;
use std::thread;
use std::time::Duration;
use std::iter::once;
use std::error::Error; 

use winapi::shared::minwindef::{DWORD, FALSE, LPVOID, BOOL as WINBOOL};
use winapi::shared::ntdef::HANDLE;

use winapi::um::handleapi::INVALID_HANDLE_VALUE;

use winapi::um::fileapi::{CreateFileW, ReadFile, WriteFile, OPEN_EXISTING};
use winapi::um::handleapi::CloseHandle;

use winapi::um::errhandlingapi::GetLastError;

use winapi::um::namedpipeapi::{CreateNamedPipeW, ConnectNamedPipe, DisconnectNamedPipe};

use winapi::um::winnt::{GENERIC_READ, GENERIC_WRITE, FILE_SHARE_READ, FILE_SHARE_WRITE};
use winapi::um::winbase::{
    PIPE_ACCESS_DUPLEX, PIPE_TYPE_BYTE, PIPE_READMODE_BYTE,
    PIPE_WAIT, PIPE_UNLIMITED_INSTANCES
};
use winapi::shared::winerror::ERROR_BROKEN_PIPE; // Moved to shared::winerror


const PIPE_NAME: &str = "\\\\.\\pipe\\cruesader_pipe";
const BUFFER_SIZE: usize = 1024;

fn lp_w_str(s: &str) -> Vec<u16> {
    OsStr::new(s).encode_wide().chain(once(0)).collect()
}

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

fn get_last_error() -> u32 {
    unsafe { GetLastError() }
}

fn run_server() -> io::Result<()> {
    println!("[Rust Server] Creating named pipe: {}", PIPE_NAME);

    let name_wide = lp_w_str(PIPE_NAME);

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

    // Wait for client connection (non-blocking approach)
    println!("[Rust Server] Waiting for C client connection...");
    
    // Use a timeout to avoid hanging indefinitely
    let mut attempts = 0;
    let max_attempts = 50; // 5 second timeout with 100ms delays
    let mut connected = false;
    
    while attempts < max_attempts && !connected {
        thread::sleep(Duration::from_millis(100));
        
        // Check if client is connected by trying to connect
        let connect_result: WINBOOL = unsafe {
            ConnectNamedPipe(pipe_handle, ptr::null_mut())
        };
        
        if connect_result != FALSE {
            connected = true;
            break;
        }
        
        let error_code = get_last_error();
        if error_code == ERROR_BROKEN_PIPE {
            // Client disconnected before we could connect
            unsafe { CloseHandle(pipe_handle); }
            return Err(PipeError {
                message: "Client disconnected before connection".to_string(),
                error_code,
            }.into());
        }
        
        attempts += 1;
    }
    
    if !connected {
        unsafe { CloseHandle(pipe_handle); }
        return Err(PipeError {
            message: "Timeout waiting for client connection".to_string(),
            error_code: get_last_error(),
        }.into());
    }

    println!("[Rust Server] C client connected successfully.");

    let server_message = "server connected.\n";
    
    // Send message to client
    if let Err(e) = write_to_pipe(pipe_handle, server_message.as_bytes()) {
        unsafe {
            DisconnectNamedPipe(pipe_handle);
            CloseHandle(pipe_handle);
        }
        return Err(e);
    }
    println!("[Rust Server] Sent message to C client: '{}'", server_message.trim());

    // Give client time to process the message and potentially respond
    thread::sleep(Duration::from_millis(100));
    
    // Try to read response, but handle client disconnection gracefully
    println!("[Rust Server] Waiting to read response from C client...");
    match read_from_pipe(pipe_handle) {
        Ok(response) => {
            let received_message = String::from_utf8_lossy(&response);
            println!("[Rust Server] Received from C client: '{}'", received_message.trim());
            if received_message == "game_saved"{
                println!("SSSSSSSSSS scrap SSSSSSSSSS")
            }
        }
        Err(e) => {
            // This is expected if client disconnects after receiving message
            if let Some(pipe_err) = e.source().and_then(|e| e.downcast_ref::<PipeError>()) {
                if pipe_err.error_code == ERROR_BROKEN_PIPE {
                    println!("[Rust Server] C client disconnected (normal behavior after receiving message).");
                } else {
                    eprintln!("[Rust Server] Failed to read from pipe: {}", e);
                }
            } else {
                eprintln!("[Rust Server] Failed to read from pipe: {}", e);
            }
        }
    }

    // Properly clean up resources
    unsafe {
        DisconnectNamedPipe(pipe_handle);
        CloseHandle(pipe_handle);
    }
    println!("[Rust Server] recieved game data send to drop box");

    Ok(())
}

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
            error_code
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
        println!("Press Ctrl+C to exit.");
    //     thread::sleep(Duration::from_secs(2));
    }
}