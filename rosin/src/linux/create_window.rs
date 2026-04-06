use crate::{desc::WindowDesc, linux::wayland::RosinWaylandState};

use std::any::Any;
use std::sync::Arc;
use wayland_client::QueueHandle;
use wayland_client::globals::BindError;
use wayland_client::globals::GlobalList;
use wayland_client::protocol::{wl_shm, wl_subcompositor, wl_surface};
use wayland_protocols::wp::tablet::zv2::client::zwp_tablet_tool_v2;
use wayland_protocols::xdg::decoration::zv1::client::zxdg_toplevel_decoration_v1::Mode;
use wayland_protocols::xdg::decoration::zv1::client::{zxdg_decoration_manager_v1, zxdg_toplevel_decoration_v1};
use wayland_protocols::xdg::shell::client::{xdg_surface, xdg_toplevel, xdg_wm_base};
use wayland_client::Connection;
pub struct GlobalData;
#[derive(Debug, Clone)]
pub struct WindowData();
pub struct WaylandWindow {
    pub(crate) xdg_surface: xdg_surface::XdgSurface,
    pub(crate) xdg_toplevel: xdg_toplevel::XdgToplevel,
    pub(crate) surface: wl_surface::WlSurface,
    pub(crate) xdg_decoration_manager: Option<zxdg_decoration_manager_v1::ZxdgDecorationManagerV1>,
    pub(crate) toplevel_decoration: Option<zxdg_toplevel_decoration_v1::ZxdgToplevelDecorationV1>,
    pub(crate) shm: Option<wl_shm::WlShm>,
    pub(crate) subcompositor: Arc<wl_subcompositor::WlSubcompositor>,
    pub(crate) compositor: Arc<wl_compositor::WlCompositor>,
    pub(crate) tablet: Option<zwp_tablet_tool_v2::ZwpTabletToolV2>,
    pub(crate) conn: Option<Connection>
}
use wayland_client::protocol::wl_compositor;
use wayland_client::protocol::wl_seat;

pub(crate) fn create_window_wayland<S: Any + Sync + 'static>(
    _desc: &WindowDesc<S>,
    globals: &GlobalList,
    qh: &QueueHandle<RosinWaylandState<S>>,
) -> Arc<WaylandWindow> {
    let wl_compositor: wl_compositor::WlCompositor = globals.bind(qh, 1..=6, ()).unwrap();
    let surface = wl_compositor.create_surface(qh, ());

    let xdg_wm_base: xdg_wm_base::XdgWmBase = globals.bind(qh, 1..=6, ()).unwrap();

    let seat: wl_seat::WlSeat = globals.bind(qh, 1..=6, ()).unwrap();

    let freeze = qh.freeze();

    let window = Arc::new_cyclic(|_weak| {
        let xdg_surface = xdg_wm_base.get_xdg_surface(&surface, qh, ());
        let xdg_toplevel = xdg_surface.get_toplevel(qh, ());
        let xdg_decoration_manager: Result<zxdg_decoration_manager_v1::ZxdgDecorationManagerV1, BindError> = globals.bind(qh, 1..=1, ());
        surface.commit();
        let toplevel_decoration = {
            if let Ok(ref xdg_deco) = xdg_decoration_manager && false {
                let toplevel_decoration = xdg_deco.get_toplevel_decoration(&xdg_toplevel, qh, ());
                toplevel_decoration.set_mode(Mode::ServerSide);
                Some(toplevel_decoration)
            } else {
                None
            }
        };
        use wayland_protocols::wp::tablet::zv2::client::zwp_tablet_manager_v2;
        let tablet_manager: zwp_tablet_manager_v2::ZwpTabletManagerV2 = globals.bind(qh, 1..=2, ()).unwrap();
        let _tablet_seat = tablet_manager.get_tablet_seat(&seat, qh, ());
        WaylandWindow {
            xdg_surface,
            xdg_toplevel,
            surface,
            xdg_decoration_manager: xdg_decoration_manager.ok(),
            toplevel_decoration,
            shm: Some(globals.bind(qh, 1..=1, ()).unwrap()),
            subcompositor: Arc::new(globals.bind(qh, 1..=1, ()).unwrap()),
            compositor: Arc::new(wl_compositor),
            tablet: None,
            conn: None
        }
    });
    // Explicitly drop the queue freeze to allow the queue to resume work.
    drop(freeze);

    window
}
