#![allow(dead_code)]

use std::{error::Error, fs, path::PathBuf, time::SystemTime};

use libloading::{Library, Symbol};

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
        let last_modified = fs::metadata(&path)?.modified()?;
        let temp_path = path.with_extension("0");
        fs::copy(&path, &temp_path)?;

        // SAFETY: This is necessary to use a dynamic library
        let lib = unsafe { Library::new(temp_path) }?;

        Ok(Self {
            lib: Some(lib),
            lib_path: path,
            last_modified,
            temp_ext: 0,
        })
    }

    pub fn get_ext(&self) -> u32 {
        self.temp_ext
    }

    /// Reload library if it changed on disk
    pub fn poll(&mut self) -> Result<bool, Box<dyn Error>> {
        let mut reloaded = false;
        let last_modified = fs::metadata(&self.lib_path)?.modified()?;

        if last_modified > self.last_modified {
            let next_temp_ext = self.temp_ext + 1;
            let next_temp_path = self.lib_path.with_extension(next_temp_ext.to_string());

            // Copy to a new location so the compiler can overwrite the original
            // If unable to copy, the file is likely still being written
            // so just wait until next poll to unload the current library
            if fs::copy(&self.lib_path, &next_temp_path).is_ok() {
                // SAFETY: This is necessary to use a dynamic library
                unsafe {
                    self.lib = Some(Library::new(next_temp_path)?);
                }

                self.last_modified = last_modified;
                fs::remove_file(self.lib_path.with_extension(self.temp_ext.to_string()))?;
                self.temp_ext = next_temp_ext;
                reloaded = true;
            }
        }

        Ok(reloaded)
    }

    pub fn get<S>(&self, symbol: &[u8]) -> Result<Symbol<S>, Box<dyn Error>> {
        // SAFETY: This is necessary to use a dynamic library
        unsafe {
            // Unwrap is ok because lib will always be Some() until dropped
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
