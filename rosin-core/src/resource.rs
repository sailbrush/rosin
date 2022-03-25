#![forbid(unsafe_code)]

use std::{collections::HashMap, error::Error, fs, num::NonZeroUsize, time::SystemTime};

use crate::stylesheet::Stylesheet;

/// Load a CSS file. In debug builds, the file will be reloaded when modified.
#[macro_export]
macro_rules! load_css {
    ($loader:expr, $path:expr) => {
        if cfg!(debug_assertions) {
            $loader.new_dynamic_css(concat!(env!("CARGO_MANIFEST_DIR"), "/", $path)).unwrap()
        } else {
            $loader
                .new_static_css(include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/", $path)))
                .unwrap()
        }
    };
}

pub(crate) trait ParseResource {
    fn parse(text: &str) -> Self;
}

#[derive(Debug, Clone, Copy)]
pub struct StyleSheetId(NonZeroUsize);

#[derive(Debug)]
pub(crate) struct Resource<T: ParseResource> {
    pub path: Option<&'static str>,
    pub last_modified: Option<SystemTime>,
    pub data: T,
}

#[derive(Debug)]
pub struct ResourceLoader {
    style_sheet_map: HashMap<&'static str, StyleSheetId>,
    style_sheets: Vec<Resource<Stylesheet>>,
}

impl ResourceLoader {
    pub fn new() -> Self {
        Self {
            style_sheet_map: HashMap::new(),
            style_sheets: Vec::new(),
        }
    }

    pub fn new_dynamic_css(&mut self, path: &'static str) -> Result<StyleSheetId, std::io::Error> {
        if let Some(&id) = self.style_sheet_map.get(path) {
            return Ok(id);
        }

        let text = fs::read_to_string(path)?;
        let data = Stylesheet::parse(&text);

        let resource = Resource {
            path: Some(path),
            last_modified: Some(fs::metadata(&path)?.modified()?),
            data,
        };

        let id = StyleSheetId(NonZeroUsize::new(self.style_sheets.len() + 1).unwrap());
        self.style_sheets.push(resource);
        self.style_sheet_map.insert(path, id);

        Ok(id)
    }

    pub fn new_static_css(&mut self, text: &'static str) -> Result<StyleSheetId, std::io::Error> {
        if let Some(&id) = self.style_sheet_map.get(text) {
            return Ok(id);
        }

        let data = Stylesheet::parse(text);

        let resource = Resource {
            path: None,
            last_modified: None,
            data,
        };

        let id = StyleSheetId(NonZeroUsize::new(self.style_sheets.len() + 1).unwrap());
        self.style_sheets.push(resource);
        self.style_sheet_map.insert(text, id);

        Ok(id)
    }

    // Reload resources if they've been modified
    pub fn poll(&mut self) -> Result<bool, std::io::Error> {
        let mut reloaded = false;

        for style_sheet in &mut self.style_sheets {
            if let Some(path) = style_sheet.path {
                let mut should_reload = true;
                let last_modified = fs::metadata(&path)?.modified()?;

                if let Some(prev_last_modified) = style_sheet.last_modified {
                    if last_modified == prev_last_modified {
                        should_reload = false;
                    }
                }

                if should_reload {
                    style_sheet.last_modified = Some(last_modified);
                    let contents = fs::read_to_string(path)?;
                    style_sheet.data = Stylesheet::parse(&contents);
                    reloaded = true;
                }
            }
        }

        Ok(reloaded)
    }

    pub(crate) fn get_sheet(&self, id: StyleSheetId) -> &Stylesheet {
        let index = usize::from(id.0) - 1;
        &self.style_sheets[index].data
    }
}
