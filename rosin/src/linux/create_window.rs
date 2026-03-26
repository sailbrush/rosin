use crate::{desc::WindowDesc, linux::wayland::RosinWaylandState};

use std::any::Any;
use std::sync::Arc;
use wayland_client::QueueHandle;
use wayland_client::globals::GlobalList;
use wayland_client::protocol::wl_surface;
use wayland_protocols::xdg::decoration::zv1::client::zxdg_toplevel_decoration_v1::Mode;
use wayland_protocols::xdg::decoration::zv1::client::{zxdg_decoration_manager_v1, zxdg_toplevel_decoration_v1};
use wayland_protocols::xdg::shell::client::{xdg_surface, xdg_toplevel, xdg_wm_base};
use wayland_client::globals::BindError;
pub struct GlobalData;
#[derive(Debug, Clone)]
pub struct WindowData();
pub struct WaylandWindow {
    pub(crate) xdg_surface: xdg_surface::XdgSurface,
    pub(crate) xdg_toplevel: xdg_toplevel::XdgToplevel,
    pub(crate) surface: wl_surface::WlSurface,
    pub(crate) xdg_decoration_manager: Option<zxdg_decoration_manager_v1::ZxdgDecorationManagerV1>,
    pub(crate) toplevel_decoration: Option<zxdg_toplevel_decoration_v1::ZxdgToplevelDecorationV1>
}
use wayland_client::protocol::wl_seat;
use wayland_client::protocol::wl_compositor;

pub(crate) fn create_window_wayland<S: Any + Sync + 'static>(
    _desc: &WindowDesc<S>,
    globals: &GlobalList,
    qh: &QueueHandle<RosinWaylandState<S>>,
) -> Arc<WaylandWindow> {
    let wl_compositor: wl_compositor::WlCompositor = globals.bind(qh, 1..=6, ()).unwrap();
    let surface = wl_compositor.create_surface(qh, ());

    let xdg_wm_base: xdg_wm_base::XdgWmBase = globals.bind(qh, 1..=6, ()).unwrap();

    let _seat: wl_seat::WlSeat = globals.bind(qh, 1..=6, ()).unwrap();

    let freeze = qh.freeze();

    let window = Arc::new_cyclic(|_weak| {
        let xdg_surface = xdg_wm_base.get_xdg_surface(&surface, qh, ());
        let xdg_toplevel = xdg_surface.get_toplevel(qh, ());
        let xdg_decoration_manager: Result<zxdg_decoration_manager_v1::ZxdgDecorationManagerV1, BindError> = globals.bind(qh, 1..=1, ());
        surface.commit();
        let toplevel_decoration = {
        
            if let Ok(ref xdg_deco) = xdg_decoration_manager {
                let toplevel_decoration = xdg_deco.get_toplevel_decoration(&xdg_toplevel, qh, ());
                toplevel_decoration.set_mode(Mode::ServerSide);
                Some(toplevel_decoration)
            }
            else {
                None
            }
        };

        WaylandWindow {
            xdg_surface,
            xdg_toplevel,
            surface,
            xdg_decoration_manager: xdg_decoration_manager.ok(),
            toplevel_decoration
        }
    });
    // Explicitly drop the queue freeze to allow the queue to resume work.
    drop(freeze);

    window
}
