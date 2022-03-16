#![forbid(unsafe_code)]

use std::{error::Error, num::NonZeroUsize};

use crate::stylesheet::Stylesheet;
use crate::tree::ArrayNode;

/// Load a CSS file. In debug builds, the file will be reloaded when modified.
#[macro_export]
macro_rules! load_css {
    ($loader:expr, $path:expr) => {
        if cfg!(debug_assertions) {
            $loader.new_dynamic_css(concat!(env!("CARGO_MANIFEST_DIR"), "/", $path))
        } else {
            $loader.new_static_css(include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/", $path)))
        }
    };
}

#[derive(Debug, Clone, Copy)]
pub struct StyleSheetId(NonZeroUsize);

#[derive(Debug)]
pub struct ResourceLoader {
    style_sheets: Vec<Stylesheet>,
}

impl ResourceLoader {
    pub fn new() -> Self {
        Self { style_sheets: Vec::new() }
    }

    pub fn new_dynamic_css(&mut self, path: &'static str) -> StyleSheetId {
        let sheet = Stylesheet::new_dynamic(path);
        let id = self.style_sheets.len();
        self.style_sheets.push(sheet);
        StyleSheetId(NonZeroUsize::new(id + 1).unwrap())
    }

    pub fn new_static_css(&mut self, text: &'static str) -> StyleSheetId {
        let sheet = Stylesheet::new_static(text);
        let id = self.style_sheets.len();
        self.style_sheets.push(sheet);
        StyleSheetId(NonZeroUsize::new(id + 1).unwrap())
    }

    pub fn poll(&mut self) -> Result<bool, Box<dyn Error>> {
        let mut result = false;
        for sheet in &mut self.style_sheets {
            result |= sheet.poll()?;
        }
        Ok(result)
    }

    pub(crate) fn get_sheet(&self, id: StyleSheetId) -> &Stylesheet {
        let index = usize::from(id.0) - 1;
        &self.style_sheets[index]
    }

    pub(crate) fn apply_style<S>(&self, tree: &mut [ArrayNode<S>]) {
        self.style_sheets[0].apply_style(tree);
    }
}
