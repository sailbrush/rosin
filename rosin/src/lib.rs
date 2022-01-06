#[cfg(all(debug_assertions, feature = "hot-reload"))]
mod libloader;

#[cfg(not(all(debug_assertions, feature = "hot-reload")))]
mod libloader {
    pub(crate) struct LibLoader {}
}

mod app;
mod view;
mod window;

/// Basic set of widgets
pub mod widgets {
    pub use rosin_core::widgets::*;
}

/// The public API
pub mod prelude {
    pub use crate::app::*;
    pub use crate::new_view;
    pub use crate::view::*;
    pub use crate::window::*;
    pub use rosin_core::prelude::*;
}
