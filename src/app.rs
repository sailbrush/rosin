#![forbid(unsafe_code)]

use crate::libloader::LibLoader;
#[cfg(all(debug_assertions, feature = "hot-reload"))]
use crate::libloader::DYLIB_EXT;

use crate::prelude::*;
use crate::style::*;
use crate::window::*;

#[cfg(all(debug_assertions, feature = "hot-reload"))]
use std::{env, path::Path};

use std::{error, fmt::Debug, mem, time::Duration, time::Instant};

use glutin::{
    event::*,
    event_loop::{ControlFlow, EventLoop},
    window::WindowId,
};

#[non_exhaustive]
#[derive(Debug, PartialEq, Eq)]
pub enum On {
    MouseDown,
    MouseUp,
    Hover,

    Change, // Can be used by widgets to signal that they have changed
    Focus,
    Blur, // TODO - cache id on focus, so blur doesn't have to search
}

#[derive(Debug, Copy, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub enum Stage {
    Idle = 0,
    Paint = 1,
    Layout = 2,
    Build = 3,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StopTask {
    Yes,
    No,
}

// This is a hack until trait aliases stabilize
pub trait EventCallback<T>: 'static + Fn(&mut T, &mut App<T>) -> Stage {}
impl<F, T> EventCallback<T> for F where F: 'static + Fn(&mut T, &mut App<T>) -> Stage {}

pub trait StyleCallback<T>: 'static + Fn(&T, &mut Style) {}
impl<F, T> StyleCallback<T> for F where F: 'static + Fn(&T, &mut Style) {}

pub trait TaskCallback<T>: 'static + Fn(&mut T, &mut App<T>) -> (Stage, StopTask) {}
impl<F, T> TaskCallback<T> for F where F: 'static + Fn(&mut T, &mut App<T>) -> (Stage, StopTask) {}

pub trait AnimCallback<T>: 'static + Fn(&mut T, Duration) -> (Stage, StopTask) {}
impl<F, T> AnimCallback<T> for F where F: 'static + Fn(&mut T, Duration) -> (Stage, StopTask) {}

pub type ViewCallback<T> = fn(&T) -> Node<T>;

struct Task<T: 'static> {
    window_id: Option<WindowId>,
    last_run: Instant,
    frequency: Duration,
    callback: Box<dyn TaskCallback<T>>,
}

pub struct AppLauncher<T: 'static>(App<T>);

impl<T: 'static> Default for AppLauncher<T> {
    fn default() -> Self {
        AppLauncher(App::new())
    }
}

impl<T: 'static> AppLauncher<T> {
    pub fn add_window(mut self, desc: WindowDesc<T>) -> Self {
        self.0.add_window(desc);
        self
    }

    pub fn use_style(mut self, stylesheet: Stylesheet) -> Self {
        self.0.use_style(stylesheet);
        self
    }

    pub fn add_font_bytes(mut self, id: u32, data: &[u8]) -> Self {
        self.0.add_font_bytes(id, data);
        self
    }

    // TODO add_anim_task

    // Similar to setInterval in JS
    pub fn add_task(mut self, window_id: Option<WindowId>, frequency: Duration, callback: impl TaskCallback<T>) -> Self {
        self.0.add_task(window_id, frequency, callback);
        self
    }

    pub fn run(self, state: T) -> Result<(), Box<dyn error::Error>> {
        self.0.run(state)
    }
}

pub struct App<T: 'static> {
    event_loop: Option<EventLoop<()>>,
    loader: LibLoader,
    new_windows: Vec<WindowDesc<T>>,
    windows: Vec<(WindowId, RosinWindow<T>)>,
    current_window: Option<WindowId>,
    stylesheet: Stylesheet,
    tasks: Vec<Task<T>>,
    fonts: Vec<(u32, Vec<u8>)>,
}

// TODO add event_filters and event_handlers?
// Need some way to access raw events for pen pressure, etc
impl<T: 'static> App<T> {
    fn new() -> Self {
        #[cfg(all(debug_assertions, feature = "hot-reload"))]
        let loader = {
            // TODO - can probably set an env variable in a build script or something
            // Use the name of the current binary to find the library
            let mut exe = env::args().next().unwrap();
            exe.push('_');
            let lib_path = env::current_dir().unwrap().join(Path::new(&exe).with_extension(DYLIB_EXT));
            LibLoader::new(lib_path).expect("[Rosin] Hot-reload: Failed to init")
        };

        #[cfg(not(all(debug_assertions, feature = "hot-reload")))]
        let loader = LibLoader {};

        Self {
            event_loop: Some(EventLoop::new()),
            loader,
            windows: Vec::new(),
            new_windows: Vec::new(),
            current_window: None,
            stylesheet: Stylesheet::default(),
            tasks: Vec::new(),
            fonts: Vec::new(),
        }
    }

    // TODO - need some way to get the id of a new window
    pub fn add_window(&mut self, desc: WindowDesc<T>) {
        self.new_windows.push(desc);
    }

    pub fn use_style(&mut self, stylesheet: Stylesheet) {
        self.stylesheet = stylesheet;
    }

    pub fn add_font_bytes(&mut self, id: u32, data: &[u8]) {
        self.fonts.push((id, data.into()));
    }

    // TODO add_anim_task

    // Similar to setInterval in JS
    // TODO - do we really need the ability to associate tasks with a particular window? Maybe event callbacks should be able to update particular windows, too
    pub fn add_task(&mut self, window_id: Option<WindowId>, frequency: Duration, callback: impl TaskCallback<T>) {
        self.tasks.push(Task {
            window_id,
            last_run: Instant::now(),
            frequency: Duration::from_millis(10).max(frequency),
            callback: Box::new(callback),
        });
    }

    pub fn current_window(&self) -> Option<WindowId> {
        self.current_window
    }

    // TODO - trigger a change event on self, and every ancestor node (need self for when a widget has only one node)
    // NOTE - this is for client code to be able to respond to events emitted by widgets, which keeps business logic in the view function
    pub fn emit_change(&mut self) {
        // make sure to stop infinite loops of change handlers emitting changes
        // probably only one event per frame, so no need to batch them up
        todo!();
    }

    pub fn focus_on(&mut self, _key: Key) {
        todo!();
    }

    // Avoids linear searching through all nodes
    // Is this really needed?
    pub fn focus_on_ancestor(&mut self, _key: Key) {
        todo!();
    }

    pub fn blur(&mut self) {
        todo!();
    }

    pub fn run(mut self, mut state: T) -> Result<(), Box<dyn error::Error>> {
        if self.new_windows.is_empty() {
            return Err("[Rosin] No windows".into());
        }

        #[cfg(debug_assertions)]
        #[allow(unused_mut)]
        self.add_task(None, Duration::from_millis(100), |_: &mut T, app: &mut App<T>| {
            let mut stage = match app.stylesheet.poll() {
                Ok(true) => Stage::Build,
                Ok(false) => Stage::Idle,
                Err(error) => {
                    eprintln!("[Rosin] Failed to reload stylesheet: {:?} Error: {:?}", app.stylesheet.path, error);
                    Stage::Idle
                }
            };

            #[cfg(feature = "hot-reload")]
            match app.loader.poll() {
                Ok(true) => stage = Stage::Build,
                Err(error) => {
                    eprintln!("[Rosin] Failed to hot-reload. Error: {:?}", error);
                }
                _ => (),
            }

            (stage, StopTask::No)
        });

        let mut active_tasks = Vec::new();
        let mut stopped_task_ids = Vec::new();

        //TODO what to do about unwraps in the event loop? Can't return error...
        let event_loop = self.event_loop.take().unwrap();
        event_loop.run(move |event, event_loop, control_flow| {
            // Run tasks
            // TODO - find a better place to run them. In response to which sytem events?
            if self.tasks.is_empty() {
                *control_flow = ControlFlow::Wait;
            } else {
                mem::swap(&mut self.tasks, &mut active_tasks);

                let mut next_update = Instant::now() + Duration::from_secs(3600);
                let mut new_stage = Stage::Idle;

                // TODO - save control_flow, and only loop through tasks if one is due for update
                for (i, task) in active_tasks.iter_mut().enumerate() {
                    if Instant::now().duration_since(task.last_run) >= task.frequency {
                        task.last_run = Instant::now();
                        let (stage, stoptask) = (task.callback)(&mut state, &mut self);
                        if let Some(window_id) = task.window_id {
                            self.windows
                                .iter_mut()
                                .find(|(id, _)| *id == window_id)
                                .expect("[Rosin] Window not found") // TODO - should log an error and continue - need to do an audit to remove panics
                                .1
                                .update_stage(stage);
                        } else {
                            new_stage = new_stage.max(stage);
                        }

                        if stoptask == StopTask::Yes {
                            stopped_task_ids.push(i);
                            continue;
                        }
                    }
                    next_update = next_update.min(task.last_run + task.frequency);
                }

                stopped_task_ids.sort_unstable();
                for id in stopped_task_ids.drain(..).rev() {
                    active_tasks.swap_remove(id);
                }

                self.tasks.append(&mut active_tasks);

                for (_, window) in self.windows.iter_mut() {
                    window.update_stage(new_stage);
                }

                *control_flow = ControlFlow::WaitUntil(next_update);
            }

            // Handle Events
            match event {
                Event::WindowEvent { event, window_id } => {
                    match event {
                        WindowEvent::Resized(physical_size) => {
                            self.windows
                                .iter_mut()
                                .find(|(id, _)| *id == window_id)
                                .expect("[Rosin] Window not found")
                                .1
                                .resize(physical_size);
                        }
                        WindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
                            self.windows
                                .iter_mut()
                                .find(|(id, _)| *id == window_id)
                                .expect("[Rosin] Window not found")
                                .1
                                .resize(*new_inner_size);
                        }
                        WindowEvent::CloseRequested => {
                            // Remove any tasks associated with the closing window
                            self.tasks
                                .retain(|task| if let Some(id) = task.window_id { id != window_id } else { true });

                            // TODO - Remove anim tasks

                            // Drops the window, causing it to close.
                            self.windows.retain(|(id, _)| *id != window_id);

                            if self.windows.is_empty() {
                                *control_flow = ControlFlow::Exit;
                                return;
                            }
                        }
                        _ => {}
                    }
                }
                Event::RedrawRequested(window_id) => {
                    self.windows
                        .iter_mut()
                        .find(|(id, _)| *id == window_id)
                        .expect("[Rosin] Window not found")
                        .1
                        .redraw(&state, &self.stylesheet, &self.loader)
                        .unwrap();
                }
                _ => {}
            }

            // Build new windows
            for desc in self.new_windows.drain(..) {
                let mut window = RosinWindow::new(desc, event_loop).unwrap();
                // TODO - handle loading and unloading fonts correctly
                // Currently, a window only has access to the fonts that are loaded before it's created
                for (id, data) in &self.fonts {
                    window.add_font_bytes(*id, data).unwrap();
                }
                self.windows.push((window.id(), window));
            }
        });
    }
}
