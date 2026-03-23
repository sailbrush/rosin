use crate::{desc::WindowDesc, linux::wayland::RosinWaylandState};

use std::any::Any;
use std::ops::Deref;
use wayland_client::QueueHandle;
use wayland_client::globals::GlobalList;
use x11rb::{
    errors::ReplyOrIdError,
    protocol::xproto::{AtomEnum, ColormapAlloc, CreateWindowAux, EventMask, PropMode, Visualid, WindowClass},
    wrapper::ConnectionExt,
};
use wayland_protocols::xdg::shell::client::{
    xdg_positioner, xdg_surface, xdg_toplevel, xdg_wm_base,
};
use wayland_client::protocol::wl_surface;
use wayland_protocols::xdg::decoration::zv1::client::zxdg_toplevel_decoration_v1::Mode;
use wayland_protocols::xdg::decoration::zv1::client::{
    zxdg_decoration_manager_v1, zxdg_toplevel_decoration_v1,
};
use std::sync::Arc;
pub struct GlobalData;
use std::sync::Weak;
#[derive(Debug, Clone)]
pub struct WindowData(pub(crate) Weak<WaylandWindow>);
pub struct WaylandWindow {
    pub(crate) xdg_surface: xdg_surface::XdgSurface,
    pub(crate) xdg_toplevel: xdg_toplevel::XdgToplevel,
    pub(crate) surface: wl_surface::WlSurface
}

pub(crate) fn create_window_x11<S: Any + Sync + 'static, T: x11rb::connection::Connection>(
    desc: &WindowDesc<S>,
    conn: &T,

    screen: &x11rb::protocol::xproto::Screen,
    atoms: &super::x11::AtomCollection,
    depth: u8,
    visual_id: Visualid,
) -> Result<x11rb::protocol::xproto::Window, ReplyOrIdError> {
    let window = conn.generate_id()?;
    let colormap = conn.generate_id()?;
    x11rb::protocol::xproto::ConnectionExt::create_colormap(&conn, ColormapAlloc::NONE, colormap, screen.root, visual_id)?;
    let win_aux = CreateWindowAux::new()
        .event_mask(
            EventMask::EXPOSURE
                | EventMask::STRUCTURE_NOTIFY
                | EventMask::BUTTON_PRESS
                | EventMask::BUTTON_RELEASE
                | EventMask::KEY_PRESS
                | EventMask::KEY_RELEASE
                | EventMask::POINTER_MOTION,
        )
        .background_pixel(x11rb::NONE)
        .border_pixel(screen.black_pixel)
        .colormap(colormap);
    x11rb::protocol::xproto::ConnectionExt::create_window(
        &conn,
        depth,
        window,
        screen.root,
        0,
        0,
        desc.size.width as u16,
        desc.size.height as u16,
        0,
        WindowClass::INPUT_OUTPUT,
        visual_id,
        &win_aux,
    )?;

    let title = desc.title.clone().unwrap();
    conn.change_property8(PropMode::REPLACE, window, AtomEnum::WM_NAME, AtomEnum::STRING, title.as_bytes())?;
    conn.change_property8(PropMode::REPLACE, window, atoms._NET_WM_NAME, atoms.UTF8_STRING, title.as_bytes())?;
    conn.change_property32(PropMode::REPLACE, window, atoms.WM_PROTOCOLS, AtomEnum::ATOM, [atoms.WM_DELETE_WINDOW].as_slice())?;
    conn.change_property8(PropMode::REPLACE, window, AtomEnum::WM_CLASS, AtomEnum::STRING, title.as_bytes())?;

    x11rb::protocol::xproto::ConnectionExt::map_window(&conn, window)?;
    Ok(window)
}




use wayland_client::protocol::wl_compositor;

pub(crate) fn create_window_wayland<S: Any + Sync + 'static>(desc: &WindowDesc<S>, globals: &GlobalList, qh: &QueueHandle<RosinWaylandState<S>>) ->Arc<WaylandWindow> {
    
    let wl_compositor: wl_compositor::WlCompositor = globals.bind(qh, 1..=6, ()).unwrap();
    let surface = wl_compositor.create_surface(qh, ());

    let xdg_wm_base: xdg_wm_base::XdgWmBase = globals.bind(qh, 1..=6, ()).unwrap();
    
    let freeze = qh.freeze();

        let window = Arc::new_cyclic(|weak| {
            let xdg_surface = xdg_wm_base.get_xdg_surface(
                &surface,
                qh,
                (),
            );
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
