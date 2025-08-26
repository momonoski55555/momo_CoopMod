use std::ffi::OsStr;
use std::io::{self, Write};
use std::os::windows::ffi::OsStrExt;
use std::ptr;
use std::thread;
use std::time::Duration;
use std::iter::once;
use std::error::Error;
use std::path::{Path, PathBuf};
use tokio::fs::{self, File};
use tokio::io::{AsyncReadExt, AsyncWriteExt}; // For file.write_all() and file.read_to_end
use reqwest::{Client, Url};
use serde::{Serialize, Deserialize};
use serde_json::json;

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

const DEFAULT_SAVE_PATH: &str = "./m2tweop_temp/save/saves"; // Default path relative to the executable

// Dropbox API access token provided by the user.
// In a production environment, this should be loaded securely (e.g., environment variable, config file).
const DROPBOX_API_TOKEN: &str = "sl.u.AF_QQRgHSkhBS50JMiwhGUpAR-VtV9JFbro1kK-bKsenaO2bPlgTSCHr64PsL_PKVch7RgRBhueY4M0AWOuxBE7LWNI_C8O9-AMsRqgvrmCb8yBFvWjKIeQUcfg_OQ09ScnBnKk_hCYscLLcYbwmOR2H3VYNrdzcaCtvRa-lCC4hUEI79MLC-v-bSQ2GOdDQAQfEGYIljE41JdVljEwU0fhXqjXRzNrLVYsk9SRh-6w7MMfhH3RIXp3h0FwyUBDqg8vvWN8ESuDFCHG0dMtH4Pn9Hs1gHWoNuB1Xakk3OKW29lRBc3c87pbu6uCm_7SkRdMhKfJANwPz7iYkVSSk3F1U6ZcjgQlTrGH1gxhzj8tWQeTiTLjLKalFXvEy_fYt_Nz36lG-Ok7pL2vcwupeN_101GBJmp89st_bb4_Ui5d8tDnnq7dLAUpUAiu7fr_Del5rsV9v7nHBZN1CmOZ0JdJz74nS0ixY58pcABVC_Y16I8vIqzRFMEKWdC-86_sZfp2fKLTikpWe_UOh1DqAtwpmyr0lVCD0K8S59wW8qlf56Ac_ZtWgqhmxY1sna__QUNH1YDNVlCVGLHEf3xFCRk49nttfnjbxGYUetX9clBrX8R43PRDblOOmDGRkDRxWg1CIqZUdGjl0HSRBBdLDwIFLq0EvayAjFQLwbTMTPKV3-_6gPppr5Wxcdy_Z8n2HUvbMi4dBlX64hYzgjzIzFmsHd_7qZ4G2IorYHqLifeJNriY3J1nbRGLBKm36M5MescVx9qqHsDl8t-_1WXBQ4TzvkTX4HR6M_8IFjlxhLUrqEDGfDSICy519B_7PLBANGK8QKGlg-HHjPnl1MPQrY-D3VrH0V8FdIovn-yoQWsXZyhsMx0WjAJcLJJwgSJqzgKpVknNtSQKvBvraUFK2j7kzykrjdkyxRxUE4T0Qj2j4EcKYPy3775z9FroPPHUKaH9YLtiLts6FJR10Tn8ER6AGMDU7lgKQAhP8imC3RxR8_M-47hY6durh62ZHUywBf_j_jDT2EmgDtjMCV7quQsdvdX6OfvhvU9czyB_7nPqAA7t_uEqpuix6slWGl12VJnkA7ESVEnS3Eg1mUWrVbsUzLGe_W9zaUhUQTX6IUMXPqiEfv9Ldu0GAfFu2b4JocefK_jG55rIXi8J0hH8Jz4MWZIwvZlQvIZXBXjzOKo1xScBQpAGyfVcpMvP2joUn6VI_msTvabpsrv7cQ9MXbvEknIZOf3ukC37u6GspLxmKemmIwLHwjqM77zF3TI8znrJf_KgdX_PxXb7DS06clo6aqzo5WNXa8i6SR8FiRehxhBISxCFRCPwgwZ2YMU-Gh6aBl7NqHXsdCJPN1F2hdHzhAFLrjAs5aS7snWva4BaDC5MZumyKeicdo6UfzxhnQ5GrjIeXEOmNt8hQCIATVR8w";

// Helper function to convert a Rust string slice to a null-terminated wide character string
// This is necessary because many Windows API functions expect UTF-16 strings.
/// Converts a Rust string slice (`&str`) to a null-terminated wide character string (`Vec<u16>`).
/// This is essential for interoperability with Windows API functions that expect
/// UTF-16 encoded strings.
fn lp_w_str(s: &str) -> Vec<u16> {
    OsStr::new(s).encode_wide().chain(once(0)).collect()
}

// Custom error type to provide more context for failed Windows API calls.
/// Custom error type to provide more context for failed Windows API calls.
/// It encapsulates a descriptive message and the Windows error code.
#[derive(Debug)]
struct PipeError {
    /// A descriptive error message.
    message: String,
    /// The Windows error code associated with the failure.
    error_code: u32,
}

impl std::fmt::Display for PipeError {
    /// Formats the `PipeError` for display, including the message and error code.
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{} (Error Code: {})", self.message, self.error_code)
    }
}

impl std::error::Error for PipeError {} // Implements the `Error` trait, allowing `PipeError` to be used as a source in `io::Error`.

impl From<PipeError> for io::Error {
    /// Converts a `PipeError` into an `io::Error` with `ErrorKind::Other`.
    fn from(err: PipeError) -> Self {
        io::Error::new(io::ErrorKind::Other, err)
    }
}

// Safe wrapper for GetLastError, which retrieves the calling thread's last-error code.
/// A safe wrapper for the Windows API `GetLastError` function.
/// Retrieves the last-error code for the calling thread, which provides
/// information about the most recent error that occurred during a Windows API call.
fn get_last_error() -> u32 {
    unsafe { GetLastError() }
}

// Server implementation compatible with C clients.
/// Implements the server-side logic for a named pipe, designed to be compatible with C clients.
/// This function creates a named pipe, waits for a client connection, exchanges messages
/// (sends a message to the client, then reads a response), and handles disconnection and cleanup.
///
/// # Returns
/// - `Ok(())` if the server operations complete successfully.
/// - `Err(io::Error)` if any Windows API call fails, returning a `PipeError` wrapped in `io::Error`.
async fn run_server() -> io::Result<()> {
    println!("[Rust Server] Creating named pipe: {}", PIPE_NAME);

    // Convert the pipe name string to a wide character string suitable for Windows API.
    let name_wide = lp_w_str(PIPE_NAME);

    // Create the named pipe using `CreateNamedPipeW`.
    // - `PIPE_ACCESS_DUPLEX`: Allows both reading and writing through the pipe.
    // - `PIPE_TYPE_BYTE | PIPE_READMODE_BYTE | PIPE_WAIT`: Configures the pipe for byte-stream
    //   operations and blocking mode, ensuring compatibility with C clients.
    // - `PIPE_UNLIMITED_INSTANCES`: Allows multiple instances of the pipe to be created.
    // - `BUFFER_SIZE`: Sets the input and output buffer sizes for the pipe.
    let pipe_handle: HANDLE = unsafe {
        CreateNamedPipeW(
            name_wide.as_ptr(),
            PIPE_ACCESS_DUPLEX,
            PIPE_TYPE_BYTE | PIPE_READMODE_BYTE | PIPE_WAIT,
            PIPE_UNLIMITED_INSTANCES,
            BUFFER_SIZE as DWORD,
            BUFFER_SIZE as DWORD,
            0, // No default timeout
            ptr::null_mut(), // No security attributes
        )
    };

    // Check if pipe creation failed.
    if pipe_handle == INVALID_HANDLE_VALUE {
        let error_code = get_last_error();
        return Err(PipeError {
            message: "Failed to create named pipe".to_string(),
            error_code,
        }.into());
    }

    println!("[Rust Server] Pipe created successfully. Waiting for C client connection...");

    // Wait for a client to connect to the pipe using `ConnectNamedPipe`.
    // This call blocks until a client opens the other end of the pipe.
    let connect_success: WINBOOL = unsafe {
        ConnectNamedPipe(pipe_handle, ptr::null_mut()) // No overlapped structure
    };

    // Check if client connection failed.
    if connect_success == FALSE {
        let error_code = get_last_error();
        unsafe { CloseHandle(pipe_handle); } // Ensure the pipe handle is closed on failure.
        return Err(PipeError {
            message: "Failed to connect client".to_string(),
            error_code,
        }.into());
    }

    println!("[Rust Server] C client connected successfully.");

    // New: Read command/message from the client
    println!("[Rust Server] Waiting to read command from C client...");
    let response_to_client: String;
    match read_from_pipe(pipe_handle) {
        Ok(client_message_bytes) => {
            let client_message = String::from_utf8_lossy(&client_message_bytes);
            println!("[Rust Server] Received from C client: '{}'", client_message.trim());

            // Process the client's command
            if client_message.trim().starts_with("DOWNLOAD ") {
                let parts: Vec<&str> = client_message.trim().splitn(2, ' ').collect();
                if parts.len() >= 2 { // Changed to >=2 to handle potential extra spaces if any
                    let download_url = parts[1];
                    let save_dir = PathBuf::from(DEFAULT_SAVE_PATH);
                    
                    println!("[Rust Server] Processing DOWNLOAD command for URL: {}", download_url);
                    match download_file(download_url, &save_dir).await {
                        Ok(file_path) => {
                            response_to_client = format!("SUCCESS: Downloaded {} to {}", download_url, file_path.display());
                        },
                        Err(e) => {
                            response_to_client = format!("ERROR: Download failed for {}: {}", download_url, e);
                        },
                    }
                } else {
                    response_to_client = "ERROR: Invalid DOWNLOAD command format. Usage: DOWNLOAD <URL>".to_string();
                }
            } else if client_message.trim().starts_with("UPLOAD ") {
                let parts: Vec<&str> = client_message.trim().splitn(3, ' ').collect(); // UPLOAD <local_path> [dropbox_path]
                if parts.len() >= 2 {
                    let local_file_str = parts[1];
                    let local_file_path = PathBuf::from(local_file_str);
                    let dropbox_dest_path = if parts.len() == 3 {
                        parts[2].to_string()
                    } else {
                        // Infer Dropbox path from local_file_path, ensure it's in a subdirectory
                        format!("/M2TWEOP_Saves/{}", local_file_path.file_name().unwrap_or(OsStr::new("uploaded_file")).to_string_lossy())
                    };
                    
                    println!("[Rust Server] Processing UPLOAD command for local file: {}", local_file_path.display());
                    match upload_file_to_dropbox(&local_file_path, &dropbox_dest_path, DROPBOX_API_TOKEN).await {
                        Ok(uploaded_path) => {
                            response_to_client = format!("SUCCESS: Uploaded {} to Dropbox path {}", local_file_path.display(), uploaded_path);
                        },
                        Err(e) => {
                            response_to_client = format!("ERROR: Upload failed for {}: {}", local_file_path.display(), e);
                        },
                    }
                } else {
                    response_to_client = "ERROR: Invalid UPLOAD command format. Usage: UPLOAD <local_filepath> [dropbox_dest_path]".to_string();
                }
            } else if client_message.trim().starts_with("DOWNLOAD_DROPBOX_SAVE ") {
                let parts: Vec<&str> = client_message.trim().splitn(2, ' ').collect(); // DOWNLOAD_DROPBOX_SAVE <save_name>
                if parts.len() == 2 {
                    let save_name = parts[1];
                    let dropbox_file_path = format!("/M2TWEOP_Saves/{}.sav", save_name); // Assuming .sav extension
                    let local_dest_dir = PathBuf::from(DEFAULT_SAVE_PATH);
                    
                    println!("[Rust Server] Processing DOWNLOAD_DROPBOX_SAVE command for '{}' from Dropbox path: {}", save_name, dropbox_file_path);
                    
                    // First, get the direct download link from Dropbox (using /files/get_temporary_link)
                    let client = reqwest::Client::new();
                    let temp_link_url = "https://api.dropboxapi.com/2/files/get_temporary_link";
                    let request_body = json!({ "path": dropbox_file_path });

                    let temp_link_response = client.post(temp_link_url)
                        .header("Authorization", format!("Bearer {}", DROPBOX_API_TOKEN))
                        .header("Content-Type", "application/json")
                        .json(&request_body)
                        .send()
                        .await
                        .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("Failed to request temporary link for Dropbox download: {}", e)))?;

                    let temp_link_status = temp_link_response.status();
                    let temp_link_response_json: serde_json::Value = temp_link_response.json().await
                        .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("Failed to parse temporary link response: {}", e)))?;

                    if !temp_link_status.is_success() {
                        eprintln!("[Rust Server] Dropbox temporary link error: Status {}, Response: {:?}", temp_link_status, temp_link_response_json);
                        response_to_client = format!("ERROR: Failed to get Dropbox download link for {}: {}", save_name, temp_link_response_json["error_summary"].as_str().unwrap_or("unknown error"));
                    } else {
                        let direct_url = temp_link_response_json["link"].as_str().unwrap_or("");
                        if !direct_url.is_empty() {
                            match download_file(direct_url, &local_dest_dir).await {
                                Ok(downloaded_path) => {
                                    response_to_client = format!("SUCCESS: Downloaded Dropbox save '{}' to '{}'. LOAD_SAVE_FILE:{}", save_name, downloaded_path.display(), save_name);
                                },
                                Err(e) => {
                                    response_to_client = format!("ERROR: Failed to download Dropbox save '{}': {}", save_name, e);
                                },
                            }
                        } else {
                            response_to_client = format!("ERROR: Dropbox temporary link was empty for save: {}", save_name);
                        }
                    }
                } else {
                    response_to_client = "ERROR: Invalid DOWNLOAD_DROPBOX_SAVE command format. Usage: DOWNLOAD_DROPBOX_SAVE <save_name>".to_string();
                }
            } else if client_message.trim().starts_with("LIST_DROPBOX_SAVES") {
                println!("[Rust Server] Processing LIST_DROPBOX_SAVES command.");
                match list_dropbox_files("/M2TWEOP_Saves/", DROPBOX_API_TOKEN).await {
                    Ok(files) => {
                        if files.is_empty() {
                            response_to_client = "SUCCESS: No Dropbox save files found.".to_string();
                        } else {
                            // Join filenames with a delimiter, e.g., commas, for Lua parsing
                            response_to_client = format!("SUCCESS: Dropbox Saves: {}", files.join(","));
                        }
                    },
                    Err(e) => {
                        response_to_client = format!("ERROR: Failed to list Dropbox saves: {}", e);
                    },
                }
            } else if client_message.trim() == "GET_STATUS" {
                 // Example of another command
                response_to_client = "STATUS: Server is operational and ready for commands.".to_string();
            }
            else {
                response_to_client = format!("ACK: Received '{}'. Unknown command.", client_message.trim());

            }
        }
        Err(e) => {
            if let Some(pipe_err) = e.source().and_then(|e| e.downcast_ref::<PipeError>()) {
                if pipe_err.error_code == ERROR_BROKEN_PIPE {
                    println!("[Rust Server] C client disconnected (broken pipe) during read.");
                } else {
                    eprintln!("[Rust Server] Failed to read from pipe: {}", e);
                }
            } else {
                eprintln!("[Rust Server] Failed to read from pipe: {}", e);
            }
            response_to_client = format!("ERROR: Failed to read client message: {}", e);
        }
    }

    // New: Send a response back to the client
    println!("[Rust Server] Sending response to C client: '{}'", response_to_client);
    if let Err(e) = write_to_pipe(pipe_handle, response_to_client.as_bytes()) {
        eprintln!("[Rust Server] Failed to write response to client: {}", e);
        // If writing fails, disconnect and close the pipe.
        unsafe {
            DisconnectNamedPipe(pipe_handle);
            CloseHandle(pipe_handle);
        }
        return Err(e); // Propagate the error.
    }
    println!("[Rust Server] Response sent.");

    // Cleanup resources by disconnecting the pipe and closing the handle.
    unsafe {
        DisconnectNamedPipe(pipe_handle);
        CloseHandle(pipe_handle);
    }
    println!("[Rust Server] Connection closed and resources cleaned up.");

    Ok(())
}

/// Downloads a file from a given URL to a specified destination path.
///
/// # Arguments
/// - `url`: The URL of the file to download.
/// - `dest_path`: The local path where the file will be saved.
///
/// # Returns
/// - `Ok(PathBuf)` if the download is successful, containing the path to the downloaded file.
/// - `Err(io::Error)` if the download fails (e.g., network error, file system error).
async fn download_file(url: &str, dest_path: &Path) -> io::Result<PathBuf> {
    println!("[Downloader] Attempting to download from: {}", url);
    println!("[Downloader] Saving to: {}", dest_path.display());

    let client = Client::new();
    let parsed_url = Url::parse(url)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, format!("Invalid URL: {}", e)))?;

    let response = client.get(parsed_url.clone())
        .send()
        .await
        .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("Failed to send request: {}", e)))?;

    if !response.status().is_success() {
        return Err(io::Error::new(io::ErrorKind::Other, format!("Failed to download file: HTTP status {}", response.status())));
    }

    // Attempt to extract filename from URL, or use a default
    let filename = parsed_url
        .path_segments()
        .and_then(|segments| segments.last())
        .unwrap_or("downloaded_file"); // Default filename

    let file_path = dest_path.join(filename);

    tokio::fs::create_dir_all(&dest_path).await?;
    let mut file = File::create(&file_path).await?;

    let content = response.bytes().await
        .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("Failed to read response bytes: {}", e)))?;

    file.write_all(&content).await?;
    file.flush().await?; // Ensure all data is written to disk

    println!("[Downloader] Downloaded successfully to: {}", file_path.display());
    Ok(file_path)
}

/// Uploads a file to Dropbox.
///
/// # Arguments
/// - `local_file_path`: The path to the local file to upload.
/// - `dropbox_dest_path`: The destination path in Dropbox (e.g., "/my_saves/save_game.dat").
/// - `token`: The Dropbox API access token.
///
/// # Returns
/// - `Ok(String)` containing the Dropbox file path if successful.
/// - `Err(io::Error)` if the upload fails.
async fn upload_file_to_dropbox(local_file_path: &Path, dropbox_dest_path: &str, token: &str) -> io::Result<String> {
    println!("[Uploader] Attempting to upload {} to Dropbox at {}", local_file_path.display(), dropbox_dest_path);

    let client = Client::new();
    let upload_url = "https://content.dropboxapi.com/2/files/upload";

    let mut file = File::open(local_file_path).await?;
    let mut contents = Vec::new();
    file.read_to_end(&mut contents).await?;

    let dropbox_api_arg = json!({
        "path": dropbox_dest_path,
        "mode": "overwrite",
        "autorename": false,
        "mute": false,
        "strict_conflict": false
    }).to_string();

    let response = client.post(upload_url)
        .header("Authorization", format!("Bearer {}", token))
        .header("Dropbox-API-Arg", dropbox_api_arg)
        .header("Content-Type", "application/octet-stream")
        .body(contents)
        .send()
        .await
        .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("Failed to send upload request to Dropbox: {}", e)))?;

    let status = response.status();
    let response_text = response.text().await
        .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("Failed to read Dropbox response: {}", e)))?;

    if !status.is_success() {
        eprintln!("[Uploader] Dropbox API Error: Status {}, Response: {}", status, response_text);
        return Err(io::Error::new(io::ErrorKind::Other, format!("Failed to upload to Dropbox: HTTP status {}", status)));
    }

    println!("[Uploader] Successfully uploaded to Dropbox: {}", response_text);
    // You might want to parse response_text to get the actual file path or ID if needed
    Ok(dropbox_dest_path.to_string())
}

/// Lists files in a specified Dropbox folder.
///
/// # Arguments
/// - `path`: The path to the folder in Dropbox (e.g., "/my_saves").
/// - `token`: The Dropbox API access token.
///
/// # Returns
/// - `Ok(Vec<String>)` containing a list of filenames (basename only) if successful.
/// - `Err(io::Error)` if the API call fails.
async fn list_dropbox_files(path: &str, token: &str) -> io::Result<Vec<String>> {
    println!("[Dropbox Lister] Listing files in Dropbox path: {}", path);

    let client = Client::new();
    let list_folder_url = "https://api.dropboxapi.com/2/files/list_folder";
    let request_body = json!({
        "path": path,
        "recursive": false, // Only list files directly in the specified folder
        "include_media_info": false,
        "include_deleted": false,
        "include_has_explicit_shared_members": false,
        "include_mounted_folders": true,
        "include_non_downloadable_files": false
    });

    let response = client.post(list_folder_url)
        .header("Authorization", format!("Bearer {}", token))
        .header("Content-Type", "application/json")
        .json(&request_body)
        .send()
        .await
        .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("Failed to send list_folder request to Dropbox: {}", e)))?;

    let status = response.status();
    let response_json: serde_json::Value = response.json().await
        .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("Failed to parse Dropbox list_folder response: {}", e)))?;

    if !status.is_success() {
        eprintln!("[Dropbox Lister] Dropbox API Error: Status {}, Response: {:?}", status, response_json);
        return Err(io::Error::new(io::ErrorKind::Other, format!("Failed to list Dropbox files: HTTP status {}, error: {}", status, response_json["error_summary"].as_str().unwrap_or("unknown error"))));
    }

    let entries = response_json["entries"].as_array()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "Dropbox response 'entries' field is not an array"))?;

    let filenames: Vec<String> = entries.iter()
        .filter_map(|entry| entry["name"].as_str().map(|s| s.to_string()))
        .collect();

    println!("[Dropbox Lister] Found files: {:?}", filenames);
    Ok(filenames)
}

/// Helper function for writing data to a named pipe with robust error handling.
///
/// # Arguments
/// - `pipe_handle`: The `HANDLE` to the opened named pipe.
/// - `data`: A byte slice (`&[u8]`) containing the data to be written.
///
/// # Returns
/// - `Ok(())` if the data is successfully written to the pipe.
/// - `Err(io::Error)` if `WriteFile` fails or an incomplete write occurs,
///   returning a `PipeError` wrapped in `io::Error`.
fn write_to_pipe(pipe_handle: HANDLE, data: &[u8]) -> io::Result<()> {
    let mut bytes_written: DWORD = 0;

    // Call the Windows API `WriteFile` function to write data to the pipe.
    // - `pipe_handle`: Handle to the pipe.
    // - `data.as_ptr() as LPVOID`: Pointer to the buffer containing the data to write.
    // - `data.len() as DWORD`: Number of bytes to write.
    // - `&mut bytes_written`: A pointer to a `DWORD` that receives the number of bytes written.
    // - `ptr::null_mut()`: No overlapped structure needed for synchronous operations.
    let write_success: WINBOOL = unsafe {
        WriteFile(
            pipe_handle,
            data.as_ptr() as LPVOID,
            data.len() as DWORD,
            &mut bytes_written,
            ptr::null_mut(),
        )
    };

    // Check if the write operation failed or if not all bytes were written.
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
/// Helper function for reading data from a named pipe into a buffer.
/// It attempts to read up to `BUFFER_SIZE` bytes from the pipe.
///
/// # Arguments
/// - `pipe_handle`: The `HANDLE` to the opened named pipe.
///
/// # Returns
/// - `Ok(Vec<u8>)` containing the data read from the pipe if successful.
/// - `Err(io::Error)` if `ReadFile` fails, returning a `PipeError` wrapped in `io::Error`.
fn read_from_pipe(pipe_handle: HANDLE) -> io::Result<Vec<u8>> {
    let mut buffer: [u8; BUFFER_SIZE] = [0; BUFFER_SIZE];
    let mut bytes_read: DWORD = 0;

    // Call the Windows API `ReadFile` function to read data from the pipe.
    // - `pipe_handle`: Handle to the pipe.
    // - `buffer.as_mut_ptr() as LPVOID`: Pointer to the buffer to receive the data.
    // - `BUFFER_SIZE as DWORD`: Maximum number of bytes to read.
    // - `&mut bytes_read`: A pointer to a `DWORD` that receives the number of bytes read.
    // - `ptr::null_mut()`: No overlapped structure needed for synchronous operations.
    let read_success: WINBOOL = unsafe {
        ReadFile(
            pipe_handle,
            buffer.as_mut_ptr() as LPVOID,
            BUFFER_SIZE as DWORD,
            &mut bytes_read,
            ptr::null_mut(),
        )
    };

    // Check if the read operation failed.
    if read_success == FALSE {
        let error_code = get_last_error();
        return Err(PipeError {
            message: "Failed to read from pipe".to_string(),
            error_code,
        }.into());
    }

    // Return the read bytes as a `Vec<u8>`.
    Ok(buffer[..bytes_read as usize].to_vec())
}

// The main function, which creates and runs the server in a continuous loop.
/// The main function, serving as the entry point for the Rust named pipe server application.
/// It continuously runs the `run_server` function in a loop, allowing it to handle
/// multiple client connections sequentially.
///
/// The server prints instructions for client compatibility and waits for a brief
/// period between server instances.
///
/// # Returns
/// - `Ok(())` if the main loop is exited cleanly (e.g., via Ctrl+C).
/// - `Err(io::Error)` if an unrecoverable error occurs within a server instance.
#[tokio::main] // Marks the main function as an asynchronous entry point using tokio runtime.
async fn main() -> io::Result<()> {
    println!("=== Rust Named Pipe Server and Downloader ===");
    println!("This server is compatible with C clients.");
    println!("Compile the C client and run it after starting this server.");
    println!();

    // Example download usage (can be triggered by pipe message or command-line later)
    let download_url = "https://www.dropbox.com/scl/fi/f0c3a2mpr7cv72j0b4l1n/example.txt?rlkey=abcdef1234567890&dl=1"; // Replace with your actual Dropbox public link
    let save_dir = PathBuf::from(DEFAULT_SAVE_PATH);

    match download_file(download_url, &save_dir).await {
        Ok(path) => println!("Application: Successfully downloaded file to {}", path.display()),
        Err(e) => eprintln!("Application: Failed to download file: {}", e),
    }

    println!();
    // The server runs in an infinite loop to handle multiple client connections sequentially.
    // Each iteration attempts to set up and run a new pipe server instance.
    loop {
        println!("Starting new server instance...");
        // Now run_server is async, so we await it.
        match run_server().await {
            Ok(()) => {
                println!("Server instance completed successfully.");
            }
            Err(e) => {
                eprintln!("Server instance error: {}", e);
            }
        }
        
        println!();
        println!("Waiting 2 seconds before starting next server instance...");
        println!("Press Ctrl+C to exit the server.");
        thread::sleep(Duration::from_secs(2)); // Pause to prevent rapid re-creation on error.
    }
}
