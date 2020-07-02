use libloading::{Library, Symbol};
use std::{error::Error, fs, path::PathBuf, time::SystemTime};

#[cfg(target_os = "windows")]
pub const DYLIB_EXT: &str = "dll";

#[cfg(target_os = "macos")]
pub const DYLIB_EXT: &str = "dylib";

#[cfg(all(not(target_os = "windows"), not(target_os = "macos")))]
pub const DYLIB_EXT: &str = "so";

pub(crate) struct LibLoader {
    lib: Option<Library>,
    lib_path: PathBuf,
    last_modified: SystemTime,
    temp_ext: u32,
}

impl LibLoader {
    pub fn new(path: PathBuf) -> Result<Self, Box<dyn Error>> {
        let metadata = fs::metadata(&path)?;
        let last_modified = metadata.modified()?;
        let temp_path = path.with_extension("0");
        fs::copy(&path, &temp_path)?;
        let lib = Library::new(temp_path)?;

        Ok(Self {
            lib: Some(lib),
            lib_path: path,
            last_modified,
            temp_ext: 0,
        })
    }

    /// Reload library if it changed on disk
    pub fn poll(&mut self) -> Result<bool, Box<dyn Error>> {
        let mut changed = false;
        let metadata = fs::metadata(&self.lib_path)?;
        let last_modified = metadata.modified()?;

        if last_modified > self.last_modified {
            let next_temp_ext = self.temp_ext + 1;
            let next_temp_path = self.lib_path.with_extension(next_temp_ext.to_string());

            // Copy to a new location so the compiler can overwrite the original
            // If unable to copy, the file is likely still being written to
            // So, just wait until next poll to unload the current library
            if fs::copy(&self.lib_path, &next_temp_path).is_ok() {
                self.last_modified = last_modified;
                self.lib = Some(Library::new(next_temp_path)?);
                fs::remove_file(self.lib_path.with_extension(self.temp_ext.to_string()))?;
                self.temp_ext = next_temp_ext;
                changed = true;
            }
        }

        Ok(changed)
    }

    pub fn get<S>(&self, symbol: &[u8]) -> Result<Symbol<S>, Box<dyn Error>> {
        // Unsafe is necessary to load from a dynamic library
        unsafe {
            // Unwrap is ok because lib *should* always be Some() until dropped
            Ok(self.lib.as_ref().unwrap().get(symbol)?)
        }
    }
}

impl Drop for LibLoader {
    fn drop(&mut self) {
        self.lib = None;
        let _ = fs::remove_file(self.lib_path.with_extension(self.temp_ext.to_string()));
    }
}
