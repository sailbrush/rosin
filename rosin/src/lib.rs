mod app;
mod libloader;
mod viewfn;
mod window;

/// Basic set of widgets
pub mod widgets {
    pub use rosin_core::widgets::*;
}

/// The public API
pub mod prelude {
    pub use crate::app::*;
    pub use crate::new_viewfn;
    pub use crate::viewfn::*;
    pub use crate::window::*;
    pub use druid_shell::WindowHandle;
    pub use rosin_core::prelude::*;
}
