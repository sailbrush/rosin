use crate::libloader::LibLoader;
use crate::libloader::DYLIB_EXT;
use std::any::Any;
use std::collections::HashMap;
use std::time::Instant;
use std::{cell::RefCell, env, error, fmt, path::Path, rc::Rc, time::Duration};

use bumpalo::{collections::Vec as BumpVec, Bump};

use druid_shell::kurbo::Vec2;
use druid_shell::piet::Piet;
use druid_shell::{Application, Cursor, KeyEvent, KeyModifiers, MouseEvent, TimerToken, WinHandler, WindowHandle};

use crate::layout::Layout;
use crate::render::render;
use crate::style::*;
use crate::view::*;
use crate::window::*;

#[non_exhaustive]
#[derive(Debug, PartialEq, Eq)]
pub enum On {
    MouseDown,
    MouseUp,
    Hover,
    Update, // Called every frame so animations can be updated
}

#[derive(Debug, PartialEq, Eq)]
pub enum Redraw {
    No,
    Yes,
}

#[derive(Debug, PartialEq, Eq)]
pub enum StopTask {
    Yes,
    No,
}

pub type TaskCallback<T> = fn(&mut T, &mut App) -> (Redraw, StopTask);

struct Task<T> {
    callback: TaskCallback<T>,
    frequency: Duration,
}

pub struct App {
    pub(crate) loader: Option<LibLoader>,
    pub(crate) stylesheet: Stylesheet,
}

impl App {
    fn new(stylesheet: Stylesheet) -> Self {
        Self {
            loader: None,
            stylesheet,
        }
    }
}

#[derive(Default)]
pub struct AppLauncher<T> {
    windows: Vec<WindowDesc<T>>,
    style: Stylesheet,
}

impl<T: std::fmt::Debug + 'static> AppLauncher<T> {
    pub fn new() -> Self {
        Self {
            windows: Vec::new(),
            style: Stylesheet::default(),
        }
    }

    pub fn add_window(mut self, desc: WindowDesc<T>) -> Self {
        self.windows.push(desc);
        self
    }

    pub fn use_style(mut self, style: Stylesheet) -> Self {
        self.style = style;
        self
    }

    pub fn launch(self, store: T) -> Result<(), Box<dyn error::Error>> {
        let mut druid_app = Application::new(None);
        let store_ref = Rc::new(RefCell::new(store));
        let mut app = App::new(self.style);

        if cfg!(debug_assertions) && cfg!(feature = "hot-reload") {
            // Use the name of the current binary to find the library
            let lib_path = env::current_dir()?.join(Path::new(&env::args().next().unwrap()).with_extension(DYLIB_EXT));
            app.loader = Some(LibLoader::new(lib_path).expect("[Rosin] Hot-reload: Failed to init"));
        }

        let app_ref: Rc<RefCell<App>> = Rc::new(RefCell::new(app));

        for mut window in self.windows {
            let handler = Box::new(RosinHandler::new(
                window.view,
                Rc::clone(&store_ref),
                Rc::clone(&app_ref),
            ));
            window.builder.set_handler(handler);
            window.builder.build()?.show();
        }

        druid_app.run();
        Ok(())
    }
}

struct RosinHandler<T> {
    handle: WindowHandle,
    size: (f64, f64),
    tasks: HashMap<TimerToken, Task<T>>,
    bump: Bump,
    should_redraw: bool,
    should_relayout: bool,
    view: View<T>,
    store: Rc<RefCell<T>>,
    app: Rc<RefCell<App>>,
}

impl<T> RosinHandler<T> {
    fn new(view: View<T>, store: Rc<RefCell<T>>, app: Rc<RefCell<App>>) -> Self {
        Self {
            handle: WindowHandle::default(),
            size: (0.0, 0.0),
            tasks: HashMap::new(),
            bump: Bump::default(),
            should_redraw: true,
            should_relayout: true,
            view,
            store,
            app,
        }
    }

    fn add_task(&mut self, frequency: Duration, callback: TaskCallback<T>) {
        let deadline = std::time::Instant::now() + frequency;
        let token = self.handle.request_timer(deadline);

        self.tasks.insert(token, Task { callback, frequency });
    }
}

impl<T: fmt::Debug> WinHandler for RosinHandler<T> {
    fn connect(&mut self, handle: &WindowHandle) {
        self.handle = handle.clone();

        if cfg!(debug_assertions) {
            self.add_task(Duration::from_millis(100), |_, app| {
                let mut redraw = match app.stylesheet.poll() {
                    Ok(true) => Redraw::Yes,
                    Ok(false) => Redraw::No,
                    Err(error) => {
                        eprintln!(
                            "[Rosin] Failed to reload stylesheet: {:?} Error: {:?}",
                            app.stylesheet.path, error
                        );
                        Redraw::No
                    }
                };

                if cfg!(feature = "hot-reload") {
                    if let Some(loader) = &mut app.loader {
                        match loader.poll() {
                            Ok(true) => redraw = Redraw::Yes,
                            Err(error) => {
                                eprintln!("[Rosin] Failed to hot-reload. Error: {:?}", error);
                            }
                            _ => (),
                        }
                    }
                }

                (redraw, StopTask::No)
            });
        }
    }

    fn paint(&mut self, piet: &mut Piet) -> bool {
        let app = self.app.borrow();
        let store = self.store.borrow();

        let time = Instant::now();

        // TODO set a bool instead of just clearing it because the cache will be needed for future events
        self.bump.reset();
        let mut tree = self.view.get(&app.loader)(&self.bump, &store)
            .finish(&self.bump)
            .unwrap();
        app.stylesheet.style(&mut tree);

        let layouts = Layout::solve(&tree, self.size).unwrap();

        let rosin_time = time.elapsed();
        let time = Instant::now();

        render(&tree, &layouts, piet);

        let piet_time = time.elapsed();
        //println!("{:#?}", tree);
        println!("{:#?} - {:#?}", rosin_time, piet_time);

        //println!("{:#?} b", self.size);
        //println!("{:#?}", std::mem::size_of::<Style>());

        false
    }

    fn command(&mut self, id: u32) {
        match id {
            0x100 => {
                self.handle.close();
                Application::quit();
            }
            _ => println!("unexpected id {}", id),
        }
    }

    fn key_down(&mut self, event: KeyEvent) -> bool {
        let deadline = std::time::Instant::now() + std::time::Duration::from_millis(500);
        let id = self.handle.request_timer(deadline);
        println!("keydown: {:?}, timer id = {:?}", event, id);
        false
    }

    fn wheel(&mut self, delta: Vec2, mods: KeyModifiers) {
        println!("mouse_wheel {:?} {:?}", delta, mods);
    }

    fn mouse_move(&mut self, event: &MouseEvent) {
        // TODO need to keep track of something for MouseEnter and MouseLeave
        self.handle.set_cursor(&Cursor::Arrow);
        /*
        if let Some(tree) = Some(&self.ui.get_tree()) {
            if let Some(layouts) = Some(&self.ui.get_layouts()) {
                let mut app = self.app.borrow_mut();
                let mut store = self.store.borrow_mut();

                let hit_node =
                    Layout::hit_test(tree, layouts, (event.pos.x as f32, event.pos.y as f32));

                // TODO hit_test should return a list of nodes, and if one with a :hover selector changes, then invalidate.
                // For now, could just invalidate on any mouse_move

                if tree[hit_node]
                    .data
                    .callbacks
                    .trigger(On::Hover, &mut store, &mut app)
                    == Redraw::Yes
                {
                    self.should_redraw = true;
                    self.handle.invalidate();
                }
            }
        }*/
    }

    fn mouse_down(&mut self, event: &MouseEvent) {
        /*if let Some(tree) = Some(&self.ui.get_tree()) {
            if let Some(layouts) = Some(&self.ui.get_layouts()) {
                let mut app = self.app.borrow_mut();
                let mut store = self.store.borrow_mut();

                let hit_node =
                    Layout::hit_test(tree, layouts, (event.pos.x as f32, event.pos.y as f32));

                if tree[hit_node]
                    .data
                    .callbacks
                    .trigger(On::MouseDown, &mut store, &mut app)
                    == Redraw::Yes
                {
                    self.should_redraw = true;
                    self.handle.invalidate();
                }
            }
        }*/
    }

    fn mouse_up(&mut self, event: &MouseEvent) {
        /*if let Some(tree) = Some(&self.ui.get_tree()) {
            if let Some(layouts) = Some(&self.ui.get_layouts()) {
                let mut app = self.app.borrow_mut();
                let mut store = self.store.borrow_mut();

                let hit_node =
                    Layout::hit_test(tree, layouts, (event.pos.x as f32, event.pos.y as f32));

                if tree[hit_node]
                    .data
                    .callbacks
                    .trigger(On::MouseUp, &mut store, &mut app)
                    == Redraw::Yes
                {
                    self.should_redraw = true;
                    self.handle.invalidate();
                }
            }
        }*/
    }

    fn timer(&mut self, id: TimerToken) {
        if let Some(task) = self.tasks.remove(&id) {
            let (r, s) = {
                let mut app = self.app.borrow_mut();
                let mut store = self.store.borrow_mut();

                (task.callback)(&mut store, &mut app)
            };

            if r == Redraw::Yes {
                self.should_redraw = true;
                self.handle.invalidate();
            }

            if s == StopTask::No {
                self.add_task(task.frequency, task.callback);
            }
        }
    }

    fn size(&mut self, width: u32, height: u32) {
        let dpi = self.handle.get_dpi();
        let dpi_scale = dpi as f64 / 96.0;
        let width_f = (width as f64) / dpi_scale;
        let height_f = (height as f64) / dpi_scale;
        self.size = (width_f, height_f);
        self.should_relayout = true;
    }

    fn destroy(&mut self) {
        Application::quit()
    }

    fn as_any(&mut self) -> &mut dyn Any {
        &mut self.handle
    }
}
