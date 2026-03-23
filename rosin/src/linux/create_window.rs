use crate::{desc::WindowDesc, linux::wayland::RosinWaylandState};

use std::any::Any;
use std::sync::Arc;
use wayland_client::QueueHandle;
use wayland_client::globals::GlobalList;
use wayland_client::protocol::wl_surface;
use wayland_protocols::xdg::shell::client::{xdg_surface, xdg_toplevel, xdg_wm_base};
pub struct GlobalData;
#[derive(Debug, Clone)]
pub struct WindowData();
pub struct WaylandWindow {
    pub(crate) xdg_surface: xdg_surface::XdgSurface,
    pub(crate) xdg_toplevel: xdg_toplevel::XdgToplevel,
    pub(crate) surface: wl_surface::WlSurface,
}


use wayland_client::protocol::wl_compositor;

pub(crate) fn create_window_wayland<S: Any + Sync + 'static>(
    _desc: &WindowDesc<S>,
    globals: &GlobalList,
    qh: &QueueHandle<RosinWaylandState<S>>,
) -> Arc<WaylandWindow> {
    let wl_compositor: wl_compositor::WlCompositor = globals.bind(qh, 1..=6, ()).unwrap();
    let surface = wl_compositor.create_surface(qh, ());

    let xdg_wm_base: xdg_wm_base::XdgWmBase = globals.bind(qh, 1..=6, ()).unwrap();

    let freeze = qh.freeze();

    let window = Arc::new_cyclic(|_weak| {
        let xdg_surface = xdg_wm_base.get_xdg_surface(&surface, qh, ());
        let xdg_toplevel = xdg_surface.get_toplevel(qh, ());

        WaylandWindow {
            xdg_surface,
            xdg_toplevel,
            surface,
        }
    });

    // Explicitly drop the queue freeze to allow the queue to resume work.
    drop(freeze);
    
    window
}
