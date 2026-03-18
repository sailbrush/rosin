use crate::{desc::WindowDesc, linux::wayland::RosinWaylandWindow};
use smithay_client_toolkit::shell::{WaylandSurface, xdg::window::Window};
use smithay_client_toolkit::{
    compositor::CompositorState,
    shell::xdg::{XdgShell, window::WindowDecorations},
};
use std::any::Any;
use std::ops::Deref;
use wayland_client::QueueHandle;
use wayland_client::globals::GlobalList;
use x11rb::{
    errors::ReplyOrIdError,
    protocol::xproto::{AtomEnum, ColormapAlloc, CreateWindowAux, EventMask, PropMode, Visualid, WindowClass},
    wrapper::ConnectionExt,
};

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

pub(crate) fn create_window_wayland<S: Any + Sync + 'static>(desc: &WindowDesc<S>, globals: &GlobalList, qh: &QueueHandle<RosinWaylandWindow<S>>) -> Window {
    let compositor_state = CompositorState::bind(globals, qh).expect("wl_compositor not available");
    let surface = compositor_state.create_surface(qh);
    let xdg_shell_state = XdgShell::bind(globals, qh).expect("xdg shell not available");
    let window = xdg_shell_state.create_window(surface, WindowDecorations::RequestServer, qh);
    window.set_title(desc.title.clone().unwrap().deref());
    window.set_app_id("rosin.default.id");
    window.set_min_size(Some((desc.min_size.unwrap_or(desc.size).width as u32, desc.min_size.unwrap_or(desc.size).height as u32)));
    window.set_max_size(Some((desc.max_size.unwrap_or(desc.size).width as u32, desc.max_size.unwrap_or(desc.size).height as u32)));

    window.commit();
    window
}
