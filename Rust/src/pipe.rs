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
use serde_derive::Deserialize;

const PIPE_NAME: &str = "\\\\.\\pipe\\coop_pipe";
const BUFFER_SIZE: usize = 1024;

struct Pipe {}

impl Pipe {

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

}
