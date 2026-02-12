mod dropbox_service;
mod gui;

use std::error::Error;
use std::ffi::OsStr;
use std::fs;
use std::io;
use std::iter::once;
use std::os::windows::ffi::OsStrExt;
use std::ptr;

use winapi::shared::minwindef::{DWORD, FALSE, LPVOID};
use winapi::shared::ntdef::HANDLE;
use winapi::shared::winerror::ERROR_BROKEN_PIPE;
use winapi::um::errhandlingapi::GetLastError;
use winapi::um::fileapi::{ReadFile, WriteFile};
use winapi::um::handleapi::{CloseHandle, INVALID_HANDLE_VALUE};
use winapi::um::namedpipeapi::{ConnectNamedPipe, CreateNamedPipeW, DisconnectNamedPipe};
use winapi::um::winbase::{
    PIPE_ACCESS_DUPLEX, PIPE_READMODE_BYTE, PIPE_TYPE_BYTE, PIPE_UNLIMITED_INSTANCES, PIPE_WAIT,
};

use crossbeam_channel::Sender;
use dropbox_service::DropboxService;
use eframe::egui;
use serde_derive::Deserialize;

const PIPE_NAME: &str = "\\\\.\\pipe\\coop_pipe";
const BUFFER_SIZE: usize = 1024;

#[derive(Deserialize, Clone)]
pub struct Config {
    pub dropbox_token: Option<String>,
    pub save_dir: Option<String>,
}

impl Config {
    pub fn load() -> Self {
        let mut cfg = fs::read_to_string("config.toml")
            .ok()
            .and_then(|content| toml::from_str(&content).ok())
            .unwrap_or(Config {
                dropbox_token: None,
                save_dir: None,
            });

        if cfg.save_dir.is_none() {
            cfg.save_dir = Self::detect_save_dir();
        }

        cfg
    }

    pub fn detect_save_dir() -> Option<String> {
        use winreg::RegKey;
        use winreg::enums::*;

        let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
        hklm.open_subkey("SOFTWARE\\WOW6432Node\\Sega\\Medieval II Total War")
            .ok()
            .and_then(|key| key.get_value::<String, _>("AppPath").ok())
            .map(|path| format!("{}\\{}", path, "mods\\crusades\\saves"))
    }
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
impl Error for PipeError {}

impl From<PipeError> for io::Error {
    fn from(err: PipeError) -> Self {
        io::Error::new(io::ErrorKind::Other, err)
    }
}

fn get_last_error() -> u32 {
    unsafe { GetLastError() }
}

fn lp_w_str(s: &str) -> Vec<u16> {
    OsStr::new(s).encode_wide().chain(once(0)).collect()
}

pub struct CooperativeServer {
    dropbox_token: String,
    save_dir: String,
    logger: Option<Sender<String>>,
}

impl CooperativeServer {
    pub fn new(token: String, save_dir: String) -> Self {
        Self {
            dropbox_token: token,
            save_dir,
            logger: None,
        }
    }

    pub fn with_logger(mut self, tx: Sender<String>) -> Self {
        self.logger = Some(tx);
        self
    }

    fn log(&self, msg: impl Into<String>) {
        let msg = msg.into();
        if let Some(tx) = &self.logger {
            let _ = tx.send(msg);
        } else {
            println!("{}", msg);
        }
    }

    pub fn run(&self) -> Result<(), Box<dyn Error>> {
        let name_wide = lp_w_str(PIPE_NAME);
        let pipe_handle = unsafe {
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
            return Err(PipeError {
                message: "Failed to create named pipe".to_string(),
                error_code: get_last_error(),
            }
            .into());
        }

        self.log("[Server] Pipe created. Waiting for client...");
        let connected = unsafe { ConnectNamedPipe(pipe_handle, ptr::null_mut()) != FALSE };

        if !connected && get_last_error() != winapi::shared::winerror::ERROR_PIPE_CONNECTED {
            unsafe { CloseHandle(pipe_handle) };
            return Err("Pipe connection failed".into());
        }

        self.log("[Server] Client connected.");
        let result = self.handle_client(pipe_handle);

        unsafe {
            DisconnectNamedPipe(pipe_handle);
            CloseHandle(pipe_handle);
        }
        result
    }

    fn handle_client(&self, pipe: HANDLE) -> Result<(), Box<dyn Error>> {
        write_to_pipe(pipe, b"SERVER_READY\n")?;

        let dbx_service = DropboxService::new(Some(self.dropbox_token.clone()))?;
        let raw_bytes = read_from_pipe(pipe)?;
        let msg = String::from_utf8_lossy(&raw_bytes).trim().to_string();

        self.log(format!("[Client]: {}", msg));

        if let Some(turn_num) = msg.strip_prefix("UPLOAD:") {
            let local_path = format!("{}\\{}", self.save_dir, "quicksave.sav");
            match dbx_service.handle_turn_upload(&local_path, turn_num) {
                Ok(_) => {
                    write_to_pipe(pipe, b"UPLOAD_OK")?;
                    self.log(format!("[Server] Uploaded turn {}", turn_num));
                }
                Err(e) => {
                    write_to_pipe(pipe, b"UPLOAD_FAIL")?;
                    self.log(format!("[Server] Upload error: {}", e));
                }
            }
        } else if let Some(turn_num) = msg.strip_prefix("DOWNLOAD:") {
            match dbx_service.download_save(turn_num, &self.save_dir) {
                Ok(path) => {
                    let response = format!("LOAD:{}", path);
                    write_to_pipe(pipe, response.as_bytes())?;
                    self.log(format!("[Server] Downloaded turn {}", turn_num));
                }
                Err(e) => {
                    write_to_pipe(pipe, b"DOWNLOAD_FAIL")?;
                    self.log(format!("[Server] Download error: {}", e));
                }
            }
        } else {
            write_to_pipe(pipe, b"UNKNOWN_CMD")?;
        }

        Ok(())
    }
}

fn write_to_pipe(pipe: HANDLE, data: &[u8]) -> io::Result<()> {
    let mut written: DWORD = 0;
    let success = unsafe {
        WriteFile(
            pipe,
            data.as_ptr() as LPVOID,
            data.len() as DWORD,
            &mut written,
            ptr::null_mut(),
        )
    };
    if success == FALSE {
        return Err(PipeError {
            message: "Write Failed".into(),
            error_code: get_last_error(),
        }
        .into());
    }
    Ok(())
}

fn read_from_pipe(pipe: HANDLE) -> io::Result<Vec<u8>> {
    let mut buffer = [0u8; BUFFER_SIZE];
    let mut read: DWORD = 0;
    let success = unsafe {
        ReadFile(
            pipe,
            buffer.as_mut_ptr() as LPVOID,
            BUFFER_SIZE as DWORD,
            &mut read,
            ptr::null_mut(),
        )
    };
    if success == FALSE {
        let err = get_last_error();
        if err == ERROR_BROKEN_PIPE {
            return Err(io::Error::new(io::ErrorKind::BrokenPipe, "Disconnected"));
        }
        return Err(PipeError {
            message: "Read Failed".into(),
            error_code: err,
        }
        .into());
    }
    Ok(buffer[..read as usize].to_vec())
}

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([400.0, 500.0]),
        ..Default::default()
    };

    eframe::run_native(
        "M2TW Seamless Co-op",
        options,
        Box::new(|cc| Ok(Box::new(gui::CoopApp::new(cc)))),
    )
}
