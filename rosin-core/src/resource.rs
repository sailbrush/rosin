#![forbid(unsafe_code)]

use std::{collections::HashMap, fs, time::SystemTime};

use crate::stylesheet::Stylesheet;

/// Load a CSS file. In debug builds, the file will be reloaded when modified.
#[macro_export]
macro_rules! load_css {
    ($loader:expr, $path:expr) => {
        if cfg!(debug_assertions) {
            $loader.new_dynamic_css(concat!(env!("CARGO_MANIFEST_DIR"), "/", $path)).unwrap()
        } else {
            $loader.new_static_css(
                concat!(env!("CARGO_MANIFEST_DIR"), "/", $path),
                include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/", $path)),
            )
        }
    };
}

#[derive(Debug)]
pub(crate) struct Resource<T> {
    pub last_modified: Option<SystemTime>,
    pub data: T,
}

#[derive(Debug, Default)]
pub struct ResourceLoader {
    style_sheets: HashMap<&'static str, Resource<Stylesheet>>,
}

impl ResourceLoader {
    pub fn new_dynamic_css(&mut self, path: &'static str) -> Result<Stylesheet, std::io::Error> {
        if let Some(stylesheet) = self.style_sheets.get(path) {
            return Ok(stylesheet.data.clone());
        }

        let text = fs::read_to_string(path)?;
        let stylesheet = Stylesheet::parse(&text);
        let resource = Resource {
            last_modified: Some(fs::metadata(&path)?.modified()?),
            data: stylesheet.clone(),
        };

        self.style_sheets.insert(path, resource);
        Ok(stylesheet)
    }

    pub fn new_static_css(&mut self, path: &'static str, text: &'static str) -> Stylesheet {
        if let Some(stylesheet) = self.style_sheets.get(path) {
            return stylesheet.data.clone();
        }

        let stylesheet = Stylesheet::parse(text);
        let resource = Resource {
            last_modified: None,
            data: stylesheet.clone(),
        };

        self.style_sheets.insert(path, resource);
        stylesheet
    }

    // Reload resources if they've been modified
    pub fn poll(&mut self) -> Result<bool, std::io::Error> {
        let mut reloaded = false;

        for (&path, style_sheet) in &mut self.style_sheets {
            if let Some(prev_last_modified) = style_sheet.last_modified {
                let last_modified = fs::metadata(&path)?.modified()?;
                if prev_last_modified != last_modified {
                    let contents = fs::read_to_string(path)?;
                    style_sheet.last_modified = Some(last_modified);
                    style_sheet.data.reparse(&contents);
                    reloaded = true;
                }
            }
        }

        Ok(reloaded)
    }
}
