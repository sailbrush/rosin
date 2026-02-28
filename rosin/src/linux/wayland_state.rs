use smithay_client_toolkit::{
    compositor::*, output::*, registry::*, seat::*, shell::xdg::window::{Window, WindowConfigure, WindowHandler}, shm::{Shm, ShmHandler}, *
};
use wayland_client::{
    Connection, QueueHandle,
    protocol::{wl_output, wl_seat, wl_surface},
};

use crate::gpu::GpuCtx;
use rosin_core::vello::{self};
use std::cell::RefCell;
use std::rc::Rc;
// based on https://github.com/Smithay/client-toolkit/blob/master/examples/wgpu.rs
pub(crate) struct RosinWaylandWindow {
    pub(crate) registry_state: RegistryState,
    pub(crate) seat_state: SeatState,
    pub(crate) output_state: OutputState,

    pub(crate) exit: bool,
    pub(crate) width: u32,
    pub(crate) height: u32,
    pub(crate) window: Window,
    pub(crate) gpu_ctx: Rc<GpuCtx>,
    pub(crate) vello_renderer: Rc<RefCell<vello::Renderer>>,
}
impl CompositorHandler for RosinWaylandWindow {
    fn scale_factor_changed(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _surface: &wl_surface::WlSurface, _new_factor: i32) {
        // Not needed for this example.
    }

    fn transform_changed(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _surface: &wl_surface::WlSurface, _new_transform: wl_output::Transform) {
        // Not needed for this example.
    }

    fn frame(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _surface: &wl_surface::WlSurface, _time: u32) {}

    fn surface_enter(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _surface: &wl_surface::WlSurface, _output: &wl_output::WlOutput) {
        // Not needed for this example.
    }

    fn surface_leave(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _surface: &wl_surface::WlSurface, _output: &wl_output::WlOutput) {
        // Not needed for this example.
    }
}

impl OutputHandler for RosinWaylandWindow {
    fn output_state(&mut self) -> &mut OutputState {
        &mut self.output_state
    }

    fn new_output(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _output: wl_output::WlOutput) {}

    fn update_output(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _output: wl_output::WlOutput) {}

    fn output_destroyed(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _output: wl_output::WlOutput) {}
}

impl WindowHandler for RosinWaylandWindow {
    fn request_close(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &Window) {
        self.exit = true;
    }

    fn configure(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _window: &Window, configure: WindowConfigure, _serial: u32) {
        let (new_width, new_height) = configure.new_size;
        self.width = new_width.map_or(self.width, |v| v.get());
        self.height = new_height.map_or(self.height, |v| v.get());

    }
}

impl SeatHandler for RosinWaylandWindow {
    fn seat_state(&mut self) -> &mut SeatState {
        &mut self.seat_state
    }

    fn new_seat(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_seat::WlSeat) {}

    fn new_capability(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _seat: wl_seat::WlSeat, _capability: Capability) {}

    fn remove_capability(&mut self, _conn: &Connection, _: &QueueHandle<Self>, _: wl_seat::WlSeat, _capability: Capability) {}

    fn remove_seat(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_seat::WlSeat) {}
}



delegate_compositor!(RosinWaylandWindow);
delegate_output!(RosinWaylandWindow);

delegate_seat!(RosinWaylandWindow);

delegate_xdg_shell!(RosinWaylandWindow);
delegate_xdg_window!(RosinWaylandWindow);

delegate_registry!(RosinWaylandWindow);

impl ProvidesRegistryState for RosinWaylandWindow {
    fn registry(&mut self) -> &mut RegistryState {
        &mut self.registry_state
    }
    registry_handlers![OutputState];
}
