
use crate::prelude::*;
use wayland_client::{protocol::wl_registry, Connection, Dispatch, QueueHandle};

struct AppData;

impl Dispatch<wl_registry::WlRegistry, ()> for AppData {
    fn event(
        _state: &mut Self,
        _: &wl_registry::WlRegistry,
        event: wl_registry::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<AppData>,
    ) {
        if let wl_registry::Event::Global { name, interface, version } = event {
            println!("[{}] {} (v{})", name, interface, version);
        }
    }
}



pub(crate) fn create_window<S: Sync + 'static>(
    desc: &WindowDesc<S>,
){
    let conn = Connection::connect_to_env().unwrap();
    
    
    let display = conn.display();
    let mut event_queue = conn.new_event_queue();
    let qh = event_queue.handle();
    let _registry = display.get_registry(&qh, ());

    

    event_queue.roundtrip(&mut AppData).unwrap();
}