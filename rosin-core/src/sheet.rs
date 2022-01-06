use std::{error::Error, num::NonZeroUsize};

use crate::{style::Stylesheet, tree::ArrayNode};

/// Load a CSS file. In debug builds, the file will be reloaded when changed.
#[macro_export]
macro_rules! load_sheet {
    ($loader:expr, $path:expr) => {
        if cfg!(debug_assertions) {
            $loader.new_dynamic(concat!(env!("CARGO_MANIFEST_DIR"), "/", $path))
        } else {
            $loader.new_static(include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/", $path)))
        }
    };
}

#[derive(Debug, Clone, Copy)]
pub struct SheetId(NonZeroUsize);

#[derive(Debug)]
pub struct SheetLoader {
    style_sheets: Vec<Stylesheet>,
}

impl SheetLoader {
    pub fn new() -> Self {
        Self { style_sheets: Vec::new() }
    }

    pub fn new_dynamic(&mut self, path: &'static str) -> SheetId {
        let sheet = Stylesheet::new_dynamic(path);
        let id = self.style_sheets.len();
        self.style_sheets.push(sheet);
        SheetId(NonZeroUsize::new(id + 1).unwrap())
    }

    pub fn new_static(&mut self, text: &'static str) -> SheetId {
        let sheet = Stylesheet::new_static(text);
        let id = self.style_sheets.len();
        self.style_sheets.push(sheet);
        SheetId(NonZeroUsize::new(id + 1).unwrap())
    }

    pub(crate) fn poll(&mut self) -> Result<bool, Box<dyn Error>> {
        let mut result = false;
        for sheet in &mut self.style_sheets {
            result &= sheet.poll()?;
        }
        Ok(result)
    }

    pub(crate) fn get_sheet(&self, id: SheetId) -> &Stylesheet {
        let index = usize::from(id.0) - 1;
        &self.style_sheets[index]
    }

    pub(crate) fn apply_style<T>(&self, tree: &mut [ArrayNode<T>]) {
        self.style_sheets[0].apply_style(tree);
    }
}
