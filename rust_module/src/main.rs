//TODO: Debloat this script remove unwanted functions remove unneed mess
//TODO: implement a way to move new generated battle's folder into the mod battles folder and simultainously uploading it dropbox
// TODO: implement a way to automatically load a save file
// TODO: implement a way to automatically to apply battle results
//TODO: rewrite the codebase without ai

mod dropbox_service; // Import the module we just made

use std::error::Error;
use std::ffi::OsStr;
use std::fs;
use std::io;
use std::iter::once;
use std::os::windows::ffi::OsStrExt;
use std::ptr;
use std::thread;
use std::time::Duration;

// WinAPI imports
use winapi::shared::minwindef::{BOOL as WINBOOL, DWORD, FALSE, LPVOID};
use winapi::shared::ntdef::HANDLE;
use winapi::shared::winerror::ERROR_BROKEN_PIPE;
use winapi::um::errhandlingapi::GetLastError;
use winapi::um::fileapi::{ReadFile, WriteFile};
use winapi::um::handleapi::{CloseHandle, INVALID_HANDLE_VALUE};
use winapi::um::namedpipeapi::{ConnectNamedPipe, CreateNamedPipeW, DisconnectNamedPipe};
use winapi::um::winbase::{
    PIPE_ACCESS_DUPLEX, PIPE_READMODE_BYTE, PIPE_TYPE_BYTE, PIPE_UNLIMITED_INSTANCES, PIPE_WAIT,
};

use dropbox_service::DropboxService;
use serde_derive::Deserialize;

const PIPE_NAME: &str = "\\\\.\\pipe\\cruesader_pipe";
const BUFFER_SIZE: usize = 1024;

// --- Config Struct ---
#[derive(Deserialize)]
struct Config {
    dropbox_token: Option<String>,
    save_dir: Option<String>,
}

impl Config {
    fn load() -> Self {
        // Try loading from config.toml
        if let Ok(content) = fs::read_to_string("config.toml") {
            if let Ok(config) = toml::from_str(&content) {
                return config;
            }
        }
        Config {
            dropbox_token: None,
            save_dir: None,
        }
    }
}

// --- Boilerplate Windows Helpers ---
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

// --- Logic Implementation ---

fn run_server(dropbox_token: Option<String>, save_dir: &str) -> Result<(), Box<dyn Error>> {
    println!("[Rust Server] Creating named pipe: {}", PIPE_NAME);
    let name_wide = lp_w_str(PIPE_NAME);

    // an indentifyer for the pipe
    let pipe_handle: HANDLE = unsafe {
        // creates a named pipe
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
// pipe error handiling
    if pipe_handle == INVALID_HANDLE_VALUE {
        return Err(Box::new(PipeError {
            message: "Failed to create named pipe".to_string(),
            error_code: get_last_error(),
        }));
    }

    println!("[Rust Server] Pipe created successfully. Waiting for C client connection...");

    // Use a timeout to avoid hanging indefinitely
    let mut attempts = 0;
    let max_attempts = 50; // 5 second timeout with 100ms delays
    let mut connected = false;

    while attempts < max_attempts && !connected {
        thread::sleep(Duration::from_millis(100));

        // Check if client is connected by trying to connect
        let connect_result: WINBOOL = unsafe { ConnectNamedPipe(pipe_handle, ptr::null_mut()) };

        if connect_result != FALSE {
            connected = true;
            break;
        }

        let error_code = get_last_error();
        if error_code == ERROR_BROKEN_PIPE {
            // Client disconnected before we could connect
            unsafe {
                CloseHandle(pipe_handle);
            }
            return Err(Box::new(PipeError {
                message: "Client disconnected before connection".to_string(),
                error_code,
            }));
        }

        attempts += 1;
    }

    if !connected {
        unsafe {
            CloseHandle(pipe_handle);
        }
        return Err(Box::new(PipeError {
            message: "Timeout waiting for client connection".to_string(),
            error_code: get_last_error(),
        }));
    }

    println!("[Rust Server] C client connected successfully.");

    // Send initial connection confirmation message to client
    let server_message = b"SERVER_READY\n";
    if let Err(e) = write_to_pipe(pipe_handle, server_message) {
        unsafe {
            DisconnectNamedPipe(pipe_handle);
            CloseHandle(pipe_handle);
        }
        return Err(e.into());
    }
    println!("[Rust Server] Sent connection confirmation to C client.");

    // Give client time to process the message
    thread::sleep(Duration::from_millis(100));

    // 1. Initialize Dropbox Service with provided token
    let dbx_service = match DropboxService::new(dropbox_token) {
        Ok(s) => s,
        // closes handle if token not found
        Err(e) => {
            eprintln!("Dropbox Init Failed: {}", e);
            unsafe {
                CloseHandle(pipe_handle);
            }
            return Err(e);
        }
    };

    // 2. Read Request from C Client
    match read_from_pipe(pipe_handle) {
        Ok(raw_bytes) => {
            let msg = String::from_utf8_lossy(&raw_bytes).trim().to_string();
            println!("[Client Request]: {}", msg);

            // LOGIC SPLIT based on Client Message
            // Expected format: "ACTION:DATA"
            // Example: "UPLOAD:5" (Turn 5 done) or "DOWNLOAD:5" (Load turn 5)

            if msg.starts_with("UPLOAD:") {
                let turn_num = msg.replace("UPLOAD:", "");
                let local_path = format!("{}\\{}", save_dir, "quicksave.sav");

                // Execute SYNC code
                println!("[Server] Processing Upload for turn {}...", turn_num);
                println!("[Server] Looking for save file at: {}", local_path);
                match dbx_service.handle_turn_upload(&local_path, &turn_num) {
                    Ok(success_msg) => {
                        println!("[Server] {}", success_msg);
                        let _ = write_to_pipe(pipe_handle, b"UPLOAD_OK");
                    }
                    Err(e) => {
                        eprintln!("[Server] Upload Error: {}", e);
                        eprintln!("[Server] Please check:");
                        eprintln!("  - File exists at: {}", local_path);
                        eprintln!("  - Directory exists: {}", save_dir);
                        eprintln!("  - You have read permissions");
                        let _ = write_to_pipe(pipe_handle, b"UPLOAD_FAIL");
                    }
                }
            } else if msg.starts_with("DOWNLOAD:") {
                let turn_num = msg.replace("DOWNLOAD:", "");

                println!("[Server] Processing Download for turn {}...", turn_num);
                println!("[Server] Will save to directory: {}", save_dir);
                match dbx_service.download_save(&turn_num, save_dir) {
                    Ok(path) => {
                        println!("[Server] Saved to {}", path);
                        // Tell C client to load this specific path
                        let response = format!("LOAD:{}", path);
                        let _ = write_to_pipe(pipe_handle, response.as_bytes());
                    }
                    Err(e) => {
                        eprintln!("[Server] Download Error: {}", e);
                        let _ = write_to_pipe(pipe_handle, b"DOWNLOAD_FAIL");
                    }
                }
            } else {
                println!("[Server] Unknown command");
                let _ = write_to_pipe(pipe_handle, b"UNKNOWN_CMD");
            }
        }
        Err(e) => {
            // Check if this is a broken pipe error (normal disconnection)
            if e.kind() == io::ErrorKind::BrokenPipe {
                println!(
                    "[Server] C client disconnected (normal behavior after receiving message)."
                );
            } else if let Some(pipe_err) = e.source().and_then(|e| e.downcast_ref::<PipeError>()) {
                if pipe_err.error_code == ERROR_BROKEN_PIPE {
                    println!("[Server] C client disconnected (pipe broken).");
                } else {
                    eprintln!("[Server] Failed to read from pipe: {}", e);
                }
            } else {
                eprintln!("[Server] Read Error: {}", e);
            }
        }
    }

    // Cleanup
    unsafe {
        DisconnectNamedPipe(pipe_handle);
        CloseHandle(pipe_handle);
    }
    Ok(())
}

// --- IO Helpers (Same as before) ---

fn write_to_pipe(pipe_handle: HANDLE, data: &[u8]) -> io::Result<()> {
    let mut bytes_written: DWORD = 0;
    let write_success = unsafe {
        WriteFile(
            pipe_handle,
            data.as_ptr() as LPVOID,
            data.len() as DWORD,
            &mut bytes_written,
            ptr::null_mut(),
        )
    };
    if write_success == FALSE {
        return Err(PipeError {
            message: "Write Failed".into(),
            error_code: get_last_error(),
        }
        .into());
    }
    Ok(())
}

fn read_from_pipe(pipe_handle: HANDLE) -> io::Result<Vec<u8>> {
    let mut buffer: [u8; BUFFER_SIZE] = [0; BUFFER_SIZE];
    let mut bytes_read: DWORD = 0;
    let read_success = unsafe {
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
        // ERROR_BROKEN_PIPE (109) means the client disconnected - handle gracefully
        if error_code == ERROR_BROKEN_PIPE {
            return Err(io::Error::new(
                io::ErrorKind::BrokenPipe,
                format!("Client disconnected (pipe broken)"),
            ));
        }
        return Err(PipeError {
            message: "Read Failed".into(),
            error_code,
        }
        .into());
    }
    Ok(buffer[..bytes_read as usize].to_vec())
}

fn main() {
    println!("=== Rust Medieval 2 Crusades Server ===");
    println!();

    // 1. Load config from config.toml if it exists
    let config = Config::load();
    let mut final_token = config.dropbox_token;
    let mut final_save_dir = config.save_dir;

    // 2. Overwrite with command-line args if present
    let args: Vec<String> = std::env::args().collect();
    if args.len() > 1 {
        println!("[Config] Using Dropbox token from command-line argument");
        final_token = Some(args[1].clone());
    }

    // 3. Fallback to Env Vars
    if final_token.is_none() {
        if let Ok(t) = std::env::var("DROPBOX_TOKEN") {
            println!("[Config] Using Dropbox token from DROPBOX_TOKEN environment variable");
            final_token = Some(t);
        }
    }

    if final_save_dir.is_none() {
        if let Ok(d) = std::env::var("SAVE_DIR") {
            final_save_dir = Some(d);
        }
    }

    // 4. Interactive Prompt if still missing
    let dropbox_token = match final_token {
        Some(t) => Some(t),
        None => {
            // Prompt user for token
            println!("Enter your Dropbox token (or press Ctrl+C to exit):");
            print!("> ");
            if io::Write::flush(&mut io::stdout()).is_err() {
                eprintln!("Error: Failed to flush stdout");
                std::process::exit(1);
            }

            let mut token = String::new();
            if io::stdin().read_line(&mut token).is_err() {
                eprintln!("Error: Failed to read token from stdin");
                std::process::exit(1);
            }
            let token = token.trim().to_string();

            if token.is_empty() {
                eprintln!("Error: No token provided");
                std::process::exit(1);
            }

            println!("[Config] Using Dropbox token from interactive input");
            Some(token)
        }
    };

    // Get save directory or use default
    let save_dir = final_save_dir.unwrap_or_else(|| {
        "C:\\Program Files (x86)\\Steam\\steamapps\\common\\Medieval II Total War\\mods\\crusades\\saves".to_string()
    });

    println!("[Config] Save directory: {}", save_dir);
    println!("[Config] Looking for file: quicksave.sav");
    println!();

    // Check if directory exists and warn if not
    if !std::path::Path::new(&save_dir).exists() {
        println!("⚠️  WARNING: Save directory does not exist!");
        println!("   Directory: {}", save_dir);
        println!("   You can set a custom path in config.toml or with SAVE_DIR env var");
        println!();
    }

    println!("Starting server...");
    println!("Press Ctrl+C to exit.");
    println!();

    loop {
        println!("Waiting for client connection...");
        match run_server(dropbox_token.clone(), &save_dir) {
            Ok(()) => {
                println!("Server completed successfully.");
            }
            Err(e) => {
                eprintln!("Server Error: {}", e);
            }
        }
        println!();
        thread::sleep(Duration::from_secs(1));
    }
}
