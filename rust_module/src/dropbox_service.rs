use rust_dropbox::client::DBXClient;
use rust_dropbox::{UploadMode, UploadOptionBuilder};
use std::error::Error;
use std::fs::File;
use std::io::Read;
use std::path::Path;

pub struct DropboxService {
    client: DBXClient,
}

impl DropboxService {
    /// Initialize the client with a token
    /// Token can be provided directly or will fall back to DROPBOX_TOKEN environment variable
    pub fn new(token: Option<String>) -> Result<Self, Box<dyn Error>> {
        let token = match token {
            Some(t) => t,
            None => {
                // Fallback to environment variable
                std::env::var("DROPBOX_TOKEN").map_err(
                    |_| "No token provided and DROPBOX_TOKEN environment variable not set",
                )?
            }
        };

        Ok(DropboxService {
            client: DBXClient::new(&token),
        })
    }

    /// Uploads a file.
    /// Logic: Renames local file -> Uploads -> Returns Success
    pub fn handle_turn_upload(
        &self,
        local_path: &str,
        turn_number: &str,
    ) -> Result<String, Box<dyn Error>> {
        let path = Path::new(local_path);

        if !path.exists() {
            return Err("Save file does not exist locally".into());
        }

        // 1. Logic: Rename file to turn number (e.g., turn_5.sav)
        let file_name = format!("turn_{}.sav", turn_number);
        let parent_dir = path.parent().unwrap_or(Path::new("./"));
        let new_local_path = parent_dir.join(&file_name);

        println!("[Service] Renaming {:?} to {:?}", path, new_local_path);
        std::fs::rename(path, &new_local_path)?;

        // 2. Read file contents
        let mut file = File::open(&new_local_path)?;
        let mut contents = Vec::new();
        file.read_to_end(&mut contents)?;

        // 3. Upload to Dropbox
        let dropbox_dest = format!("/highest_numbered_files/{}", file_name);
        println!("[Service] Uploading to {}", dropbox_dest);

        // Upload mode: Overwrite if it exists
        // Build UploadOption using UploadOptionBuilder with Overwrite mode
        let upload_option = UploadOptionBuilder::new()
            .set_upload_mode(UploadMode::Overwrite)
            .build();

        self.client
            .upload(contents, &dropbox_dest, upload_option)
            .map_err(|e| format!("Dropbox error: {:?}", e))?;

        Ok(format!("Uploaded: {}", file_name))
    }

    /// Downloads the save file.
    /// Logic: Download -> Save to disk -> Return Path
    pub fn download_save(
        &self,
        turn_number: &str,
        target_dir: &str,
    ) -> Result<String, Box<dyn Error>> {
        let file_name = format!("turn_{}.sav", turn_number);
        let dropbox_path = format!("/highest_numbered_files/{}", file_name);

        println!("[Service] Downloading {}", dropbox_path);

        // 1. Download content
        let content = self
            .client
            .download(&dropbox_path)
            .map_err(|e| format!("Dropbox error: {:?}", e))?;

        // 2. Save locally
        let save_path = Path::new(target_dir).join(&file_name);
        std::fs::write(&save_path, content)?;

        Ok(save_path.to_string_lossy().into_owned())
    }
}
