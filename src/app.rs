use std::{
    cell::RefCell,
    env,
    error::Error,
    fmt,
    fmt::Debug,
    fs,
    path::Path,
    rc::Rc,
    sync::Arc,
    sync::Weak,
    thread,
    thread::JoinHandle,
    time::{Duration, Instant},
};

use glutin::EventsLoop;
use libloading::Library;

use crate::dom::*;
use crate::style::*;
use crate::view::*;
use crate::window::{WindowBuilder, *};

pub const MAX_FRAME_TIME_MICRO: u64 = 1_000_000 / 60;

#[derive(Debug)]
pub enum On {
    Click,
    Hover,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Redraw {
    No,
    Yes,
}

#[derive(Debug, PartialEq, Eq)]
pub enum StopDaemon {
    Yes,
    No,
}

pub type Callback<T> = (fn(&mut T, app: &mut App<T>) -> Redraw);
pub type DaemonCallback<T> = (fn(&mut T, &mut App<T>) -> (Redraw, StopDaemon));
pub type EventCallback = (fn(&glutin::Event) -> Redraw);

#[derive(Default)]
pub struct CallbackList<T> {
    list: Vec<(On, Callback<T>)>,
}

impl<T> fmt::Debug for CallbackList<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "CallbackList[{}]", self.list.len())
    }
}

impl<T> CallbackList<T> {
    pub fn new() -> Self {
        CallbackList { list: Vec::new() }
    }

    pub fn insert(&mut self, event_type: On, callback: fn(&mut T, app: &mut App<T>) -> Redraw) {
        self.list.push((event_type, callback));
    }
}

pub struct Daemon<T> {
    pub callback: DaemonCallback<T>,
    pub interval: Duration,
    pub last_run: Option<Instant>,
    pub end_time: Option<Instant>,
}

pub struct Task {
    pub join_flag: Weak<()>,
    pub handle: RefCell<Option<JoinHandle<()>>>,
}

pub struct App<T> {
    pub(crate) stylesheet: Stylesheet,
    pub(crate) tasks: Vec<Task>,
    pub(crate) daemons: Rc<RefCell<Vec<Daemon<T>>>>,
    pub(crate) new_daemons: Vec<Daemon<T>>,
    pub(crate) window_mgr: WindowManager<T>,
    pub(crate) events_loop: EventsLoop,
}

impl<T> Default for App<T> {
    fn default() -> Self {
        Self {
            stylesheet: Stylesheet::default(),
            tasks: Vec::new(),
            daemons: Rc::new(RefCell::new(Vec::new())),
            new_daemons: Vec::new(),
            window_mgr: WindowManager::default(),
            events_loop: EventsLoop::new(),
        }
    }
}

impl<T> App<T> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set_style(&mut self, stylesheet: Stylesheet) {
        self.stylesheet = stylesheet;
    }

    pub fn create_window(
        &mut self,
        builder: WindowBuilder<T>,
    ) -> Result<glutin::WindowId, Box<dyn Error>> {
        self.window_mgr.create(builder, &self.events_loop)
    }

    pub fn spawn_task<F>(&mut self, func: F)
    where
        F: FnOnce() + Send + 'static,
    {
        let thread_arc = Arc::new(());
        let join_flag = Arc::downgrade(&thread_arc);

        let join_handle = thread::spawn(move || {
            let _ = thread_arc;
            func();
        });

        let task_info = Task {
            join_flag,
            handle: RefCell::new(Some(join_handle)),
        };

        self.tasks.push(task_info);
    }

    pub fn add_daemon(
        &mut self,
        interval: Option<Duration>,
        end_time: Option<Instant>,
        callback: DaemonCallback<T>,
    ) {
        self.new_daemons.push(Daemon {
            callback,
            interval: interval.unwrap_or_else(|| Duration::new(0, 0)),
            last_run: None,
            end_time,
        });
    }

    pub fn run(mut self, mut store: T) {
        println!("{:#?}", self.stylesheet);
        // DEBUG: Reload styles every 500 miliseconds
        if cfg!(debug_assertions) {
            self.add_daemon(
                Some(Duration::from_millis(500)),
                None,
                |_: &mut T, app: &mut App<T>| {
                    app.stylesheet.reload();
                    (Redraw::Yes, StopDaemon::No)
                },
            );
        }

        // DEBUG: Load app logic dynamically
        let lib_path = Path::new(&env::args().next().unwrap()).with_extension(DYLIB_EXT);
        let lib_path = env::current_dir().unwrap().join(lib_path);
        let mut temp_ext: u32 = 0;
        let mut temp_path = lib_path.with_extension(temp_ext.to_string());
        let mut lib = None;
        let mut last_modified = None;
        if cfg!(debug_assertions) {
            if let Ok(metadata) = fs::metadata(&lib_path) {
                if let Ok(modified) = metadata.modified() {
                    last_modified = Some(modified);
                    fs::copy(&lib_path, &temp_path).unwrap();
                    lib = Some(Library::new(&temp_path).unwrap());
                }
            }
        }

        // Main loop
        while !self.window_mgr.is_empty() {
            let prev_frame = Instant::now();
            let mut redraw = Redraw::No;

            // DEBUG: Reload app logic when recompiled
            if cfg!(debug_assertions) {
                if let Ok(metadata) = fs::metadata(&lib_path) {
                    if let Ok(modified) = metadata.modified() {
                        if modified > last_modified.unwrap() {
                            let next_temp_ext = temp_ext + 1;
                            let next_temp_path = lib_path.with_extension(next_temp_ext.to_string());

                            if fs::copy(&lib_path, &next_temp_path).is_ok() {
                                last_modified = Some(modified);

                                drop(lib);
                                fs::remove_file(&temp_path).unwrap();
                                temp_ext = next_temp_ext;
                                temp_path = next_temp_path;

                                lib = Some(Library::new(&temp_path).unwrap());
                                redraw = Redraw::Yes;
                            }
                        }
                    }
                }
            }

            // Handle events
            let windows = &mut self.window_mgr;
            self.events_loop.poll_events(|event| {
                match event {
                    glutin::Event::WindowEvent { event, window_id } => {
                        match event {
                            glutin::WindowEvent::CloseRequested => {
                                windows.close(window_id);
                            }
                            glutin::WindowEvent::Resized(logical_size) => {
                                windows.resize(window_id, logical_size);
                                redraw = Redraw::Yes
                            }

                            //TODO Mouse input
                            _ => redraw = Redraw::Yes,
                        }
                    }
                    _ => redraw = Redraw::Yes,
                }
            });

            // Add new daemons to active list
            let daemon_list = self.daemons.clone();
            let mut daemon_list = daemon_list.borrow_mut();
            daemon_list.append(&mut self.new_daemons);

            // Run daemons
            let now = Instant::now();
            let mut finished_daemons = Vec::new();
            for (i, daemon) in daemon_list.iter_mut().enumerate() {
                if daemon.end_time.is_some() && daemon.end_time.unwrap() <= now {
                    finished_daemons.push(i);
                } else if daemon.last_run.is_none()
                    || now.duration_since(daemon.last_run.unwrap()) >= daemon.interval
                {
                    // Call callback
                    let (r, s) = (daemon.callback)(&mut store, &mut self);
                    daemon.last_run = Some(now);
                    if r == Redraw::Yes {
                        redraw = Redraw::Yes;
                    }
                    if s == StopDaemon::Yes {
                        finished_daemons.push(i);
                    }
                }
            }
            for i in finished_daemons {
                daemon_list.remove(i);
            }

            // Join completed tasks
            let mut finished_tasks = Vec::new();
            for (i, task) in self.tasks.iter_mut().enumerate() {
                if task.join_flag.upgrade().is_none() {
                    // Get unique ownership of thread handle and join it
                    task.handle.replace(None).unwrap().join().unwrap();
                    redraw = Redraw::Yes;
                    finished_tasks.push(i);
                }
            }
            for i in finished_tasks {
                self.tasks.remove(i);
            }

            // TODO Call callbacks to update Store

            // Draw interface
            if redraw == Redraw::Yes {
                let mut doms: Vec<(glutin::WindowId, Dom<T>)> = Vec::new();
                for (id, window) in self.window_mgr.window_map.iter() {
                    let dom = (window.view.get(&lib))(&store);
                    doms.push((*id, dom));
                }
                for (id, dom) in doms.iter() {
                    self.window_mgr.draw(*id, &dom, &self.stylesheet).unwrap();
                }
            }

            // Sleep to cap framerate
            if let Some(time) =
                Duration::from_micros(MAX_FRAME_TIME_MICRO).checked_sub(prev_frame.elapsed())
            {
                thread::sleep(time);
            }
        }

        // DEBUG: Cleanup temp dynamic library
        if cfg!(debug_assertions) {
            drop(lib);
            fs::remove_file(&temp_path).unwrap();
        }
    }
}
