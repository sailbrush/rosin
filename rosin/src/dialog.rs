//! Types for configuring the behavior of a file dialog.

use std::path::PathBuf;

/// Defines a file format filter for system dialogs.
///
/// This structure is used by the system to restrict which files a user can select
/// and to provide meaningful category labels in the UI.
///
/// macOS doesn't use the `name` parameter.
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct FileDesc {
    pub name: &'static str,
    pub extensions: &'static [&'static str],
}

impl FileDesc {
    pub const ALL: FileDesc = FileDesc::new("All Files", &["*"]);

    // Documents
    pub const MARKDOWN: FileDesc = FileDesc::new("Markdown Document", &["md", "mkd", "mkdn", "mdown", "markdown"]);
    pub const PDF: FileDesc = FileDesc::new("PDF Document", &["pdf"]);
    pub const TEXT: FileDesc = FileDesc::new("Text Document", &["txt", "log", "conf"]);

    // Programming and Data
    pub const CSS: FileDesc = FileDesc::new("Style Sheet", &["css"]);
    pub const CSV: FileDesc = FileDesc::new("Comma Separated Values", &["csv"]);
    pub const HTML: FileDesc = FileDesc::new("HTML Document", &["html", "htm"]);
    pub const JSON: FileDesc = FileDesc::new("JSON File", &["json"]);
    pub const RUST: FileDesc = FileDesc::new("Rust Source", &["rs"]);
    pub const TOML: FileDesc = FileDesc::new("TOML Config", &["toml"]);
    pub const XML: FileDesc = FileDesc::new("XML File", &["xml"]);
    pub const YAML: FileDesc = FileDesc::new("YAML File", &["yaml", "yml"]);

    // Images
    pub const IMAGE: FileDesc = FileDesc::new("Image", &["png", "jpg", "jpeg", "gif", "webp", "avif"]);
    pub const SVG: FileDesc = FileDesc::new("SVG File", &["svg"]);

    // Archives
    pub const TAR: FileDesc = FileDesc::new("Tarball", &["tar", "tgz", "tbz2", "txz"]);
    pub const ZIP: FileDesc = FileDesc::new("Zip Archive", &["zip"]);

    /// Create a new [`FileDesc`]
    pub const fn new(name: &'static str, extensions: &'static [&'static str]) -> Self {
        FileDesc { name, extensions }
    }
}

/// Describes the desired behavior of a native file dialog.
#[derive(Clone, Debug)]
pub struct FileDialogOptions {
    pub(crate) allow_multiple: bool,
    pub(crate) allow_new_folders: bool,
    pub(crate) allowed_types: Option<Vec<FileDesc>>,
    pub(crate) browse_packages: bool,
    pub(crate) filename_label: Option<String>,
    pub(crate) initial_name: Option<String>,
    pub(crate) initial_path: Option<PathBuf>,
    pub(crate) pick_folders: bool,
    pub(crate) show_hidden: bool,
    pub(crate) submit_label: Option<String>,
    pub(crate) title: Option<String>,
}

impl Default for FileDialogOptions {
    fn default() -> Self {
        Self::new()
    }
}

impl FileDialogOptions {
    /// Creates a new configuration with sensible default behaviors.
    pub fn new() -> Self {
        Self {
            allow_multiple: false,
            allow_new_folders: false,
            allowed_types: None,
            browse_packages: false,
            filename_label: None,
            initial_name: None,
            initial_path: None,
            pick_folders: false,
            show_hidden: false,
            submit_label: None,
            title: None,
        }
    }

    /// Sets the main descriptive text shown by the dialog.
    ///
    /// Applies to: **Open** + **Save**
    pub fn set_title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Customizes the text on the confirmation button, such as "Save", "Import", or "Choose".
    ///
    /// Applies to: **Open** + **Save**
    pub fn set_submit_label(mut self, label: impl Into<String>) -> Self {
        self.submit_label = Some(label.into());
        self
    }

    /// Sets the directory that the dialog should show initially.
    ///
    /// - If a file path is provided, implementations may use its parent directory.
    ///
    /// Applies to **Open** + **Save**
    pub fn set_initial_path(mut self, path: impl Into<PathBuf>) -> Self {
        self.initial_path = Some(path.into());
        self
    }

    /// Enables creating new folders from within the dialog.
    ///
    /// Applies to: **Open** + **Save**
    pub fn allow_new_folders(mut self) -> Self {
        self.allow_new_folders = true;
        self
    }

    /// Forces hidden files to be visible in the dialog.
    ///
    /// Applies to: **Open** + **Save**
    pub fn show_hidden(mut self) -> Self {
        self.show_hidden = true;
        self
    }

    /// Adds a specific file format to the list of selectable types in the dialog.
    ///
    ///  - By default, all file types are allowed.
    ///  - If any entry contains `"*"` as an extension, all file types are allowed.
    ///  - If `pick_folders` is enabled, filters are ignored.
    ///  - On Windows: The first allowed type will be selected when the dialog opens.
    ///
    /// Applies to: **Open**
    pub fn allow_type(mut self, file_type: FileDesc) -> Self {
        let mut list = self.allowed_types.unwrap_or_default();
        list.push(file_type);
        self.allowed_types = Some(list);
        self
    }

    /// Enables selecting more than one item at a time.
    ///
    /// Applies to: **Open**
    pub fn allow_multiple(mut self) -> Self {
        self.allow_multiple = true;
        self
    }

    /// Changes the open dialog to select folders instead of files.
    ///
    /// - When enabled, `allow_type` is ignored.
    ///
    /// Applies to: **Open**
    pub fn pick_folders(mut self) -> Self {
        self.pick_folders = true;
        self
    }

    /// Sets the label for the filename text field, such as "Project Name".
    ///
    /// Applies to: **Save**
    pub fn set_filename_label(mut self, label: impl Into<String>) -> Self {
        self.filename_label = Some(label.into());
        self
    }

    /// Provides a suggested filename to pre-fill the dialog.
    ///
    /// Applies to: **Save**
    pub fn set_initial_name(mut self, name: impl Into<String>) -> Self {
        self.initial_name = Some(name.into());
        self
    }

    /// **macOS Only**
    ///
    /// Treats file packages, such as .app bundles, as
    /// browsable directories rather than single files.
    ///
    /// Applies to: **Open** + **Save**
    pub fn browse_packages(mut self) -> Self {
        self.browse_packages = true;
        self
    }
}
