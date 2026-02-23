use std::{
    cell::RefCell,
    env,
    ffi::OsString,
    fs,
    io::{self, Write},
    path::{Path, PathBuf},
    process::Command,
    rc::Rc,
    sync::Arc,
    time::SystemTime,
};

use libloading::Library;
use objc2::{ClassType, rc::Retained};
use objc2_app_kit::NSApp;
use objc2_foundation::{MainThreadMarker, NSObjectProtocol};
use serde::{Deserialize, Serialize};

use rosin_core::{interner::StringInterner, reactive};

use crate::{
    kurbo::{Point, Size},
    log,
    parking_lot::RwLock,
    prelude::*,
    typehash,
};

#[unsafe(no_mangle)]
pub extern "Rust" fn rosin_init_registry(registry: &'static Registry, scopes: Rc<RefCell<Vec<DependencyMap>>>, interner: Arc<RwLock<StringInterner>>) -> bool {
    Registry::set_global(registry) && reactive::try_init_read_scopes(scopes) && StringInterner::set_global(interner)
}

#[derive(Serialize, Deserialize)]
pub(crate) struct HotReloadSnapshot<S> {
    pub state: S,
    pub windows: Vec<SerializableWindowDesc>,
}

#[derive(Serialize, Deserialize)]
pub(crate) struct SerializableWindowDesc {
    pub viewfn: String,
    pub wgpufn: Option<String>,
    pub title: Option<String>,
    pub menu: Option<MenuDesc>,
    pub size: Size,
    pub min_size: Option<Size>,
    pub max_size: Option<Size>,
    pub resizeable: bool,
    pub position: Option<Point>,
    pub close_button: bool,
    pub minimize_button: bool,
    pub maximize_button: bool,
}

impl SerializableWindowDesc {
    pub fn convert<S: 'static>(self, lib: &Library) -> WindowDesc<S> {
        let view_symbol: &'static str = Box::leak(self.viewfn.into_boxed_str());

        type ViewSig<S> = for<'a, 'b> fn(&'a S, &'b mut Ui<S, WindowHandle>);

        let view_func: ViewSig<S> = unsafe {
            *lib.get::<ViewSig<S>>(view_symbol.as_bytes())
                .unwrap_or_else(|e| panic!("Failed to load view callback symbol `{}`: {e}", view_symbol))
        };

        type WgpuSig<S> = for<'a, 'b, 'c> fn(&'a S, &'b mut WgpuCtx<'c>);

        let wgpufn = self.wgpufn.map(|name| {
            let wgpu_symbol: &'static str = Box::leak(name.into_boxed_str());

            let wgpu_func: WgpuSig<S> = unsafe {
                *lib.get::<WgpuSig<S>>(wgpu_symbol.as_bytes())
                    .unwrap_or_else(|e| panic!("Failed to load wgpu callback symbol `{}`: {e}", wgpu_symbol))
            };

            (wgpu_symbol, wgpu_func).into()
        });

        WindowDesc {
            viewfn: (view_symbol, view_func).into(),
            wgpufn,
            title: self.title.map(Arc::<str>::from),
            menu: self.menu,
            size: self.size,
            min_size: self.min_size,
            max_size: self.max_size,
            resizeable: self.resizeable,
            position: self.position,
            close_button: self.close_button,
            minimize_button: self.minimize_button,
            maximize_button: self.maximize_button,
        }
    }
}

pub(crate) struct HotReloader {
    pub last_modified: SystemTime,
    pub lib: Library,
    pub ext: u32,
}

impl HotReloader {
    fn library_path() -> Option<PathBuf> {
        let binary_path = env::current_exe().ok()?;
        let file_name = binary_path.file_name()?;
        let stem = Path::new(file_name).file_stem()?;
        let mut new_name = OsString::from("lib");
        new_name.push(stem);
        new_name.push(".dylib");

        Some(binary_path.with_file_name(new_name))
    }

    fn snapshot_path() -> Option<PathBuf> {
        let exe = env::current_exe().ok()?;
        let dir = exe.parent()?;
        Some(dir.join("hot-reload-snapshot.json"))
    }

    fn write_snapshot(path: &Path, json: &str) -> io::Result<()> {
        let mut f = fs::File::create(path)?;
        f.write_all(json.as_bytes())?;
        f.sync_all()?;
        Ok(())
    }

    fn spawn_reloaded_process(snapshot_path: &Path) -> io::Result<()> {
        let exe = env::current_exe()?;
        let mut cmd = Command::new(exe);

        cmd.args(env::args_os().skip(1));
        cmd.env("ROSIN_HOT_RELOAD_SNAPSHOT", snapshot_path);

        let _child = cmd.spawn()?;
        Ok(())
    }

    #[allow(clippy::type_complexity)]
    pub fn init_registry(lib: &Library) {
        let symbol = unsafe { lib.get(b"rosin_init_registry") };
        let registry_func: libloading::Symbol<fn(&'static Registry, Rc<RefCell<Vec<DependencyMap>>>, Arc<RwLock<StringInterner>>) -> bool> = match symbol {
            Ok(s) => s,
            Err(_) => return,
        };
        if !registry_func(Registry::global(), reactive::read_scopes_rc(), StringInterner::global().clone()) {
            log::error!("Hot-reload failed to init loaded image.");
        }
    }

    pub fn new() -> Option<Self> {
        let lib_path = Self::library_path()?;

        let last_modified = match fs::metadata(&lib_path).and_then(|m| m.modified()) {
            Ok(t) => t,
            Err(_) => return None,
        };

        // Copy library to new file
        let copied_path = lib_path.with_extension("0");
        if fs::copy(&lib_path, &copied_path).is_err() {
            return None;
        }

        // Attempt to load library from new copy
        let lib = match unsafe { Library::new(&copied_path) } {
            Ok(lib) => lib,
            Err(_) => {
                // If it doesn't work, delete the copy
                let _ = fs::remove_file(&copied_path);
                return None;
            }
        };

        Self::init_registry(&lib);

        Some(Self { last_modified, lib, ext: 0 })
    }

    /// Reloads the dynamic library if it changed on disk, and updates all windows.
    pub fn reload_if_changed<S: TypeHash + Serialize>(&mut self, state: &Rc<RefCell<S>>) {
        // Must be on main thread
        let Some(mtm) = MainThreadMarker::new() else {
            return;
        };

        let Some(lib_path) = Self::library_path() else {
            return;
        };

        let last_modified = match fs::metadata(&lib_path).and_then(|m| m.modified()) {
            Ok(t) => t,
            Err(_) => return,
        };

        if self.last_modified == last_modified {
            return;
        }

        // Copy dylib to a fresh filename before loading
        let old_ext = self.ext;
        let new_ext = self.ext + 1;

        let new_path = lib_path.with_extension(new_ext.to_string());
        if fs::copy(&lib_path, &new_path).is_err() {
            return;
        }

        let new_lib = match unsafe { Library::new(&new_path) } {
            Ok(l) => l,
            Err(_) => {
                let _ = fs::remove_file(&new_path);
                return;
            }
        };

        let symbol = unsafe { new_lib.get(b"rosin_state_typehash") };
        let foreign_typehash_func: libloading::Symbol<fn(u64) -> u64> = match symbol {
            Ok(s) => s,
            Err(_) => return,
        };
        let foreign_typehash = foreign_typehash_func(typehash::DEFAULT_DEPTH);
        let local_typehash = S::get_typehash(typehash::DEFAULT_DEPTH);

        let app = NSApp(mtm);

        if foreign_typehash != local_typehash {
            // Serialize app and window states to disk, then start a new process to load it.
            let mut windows = Vec::new();
            for window in app.windows().iter() {
                let Some(view) = window.contentView() else { continue };

                if !view.isKindOfClass(crate::mac::window::RosinView::class()) {
                    continue;
                }

                // SAFETY: isKindOfClass checked above
                let rosin_view: &crate::mac::window::RosinView = unsafe { &*(Retained::as_ptr(&view) as *const crate::mac::window::RosinView) };

                windows.push(rosin_view.serializable_window_desc());
            }

            let snapshot_json = {
                let state_ref = state.borrow();
                let snapshot_data = HotReloadSnapshot { state: &*state_ref, windows };

                match crate::reactive::serde_impl::serde_scope(|| serde_json::to_string(&snapshot_data)) {
                    Ok(s) => s,
                    Err(e) => {
                        log::error!("Hot-reload failed to serialize snapshot: {e}");
                        return;
                    }
                }
            };

            let snapshot_path = match Self::snapshot_path() {
                Some(p) => p,
                None => {
                    log::error!("Hot-reload failed: could not determine snapshot path");
                    return;
                }
            };

            if let Err(e) = Self::write_snapshot(&snapshot_path, &snapshot_json) {
                log::error!("Hot-reload failed to write snapshot file: {e}");
                return;
            }

            if let Err(e) = Self::spawn_reloaded_process(&snapshot_path) {
                log::error!("Hot-reload failed to spawn reloaded process: {e}");
                return;
            }

            app.terminate(None);
        } else {
            Self::init_registry(&new_lib);

            for window in app.windows().iter() {
                let Some(view) = window.contentView() else { continue };

                if !view.isKindOfClass(crate::mac::window::RosinView::class()) {
                    continue;
                }

                // SAFETY: isKindOfClass checked above
                let rosin_view: &crate::mac::window::RosinView = unsafe { &*(Retained::as_ptr(&view) as *const crate::mac::window::RosinView) };

                rosin_view.use_library(&new_lib);
            }

            // Swap, then unload and delete the previous copy
            let old_lib = std::mem::replace(&mut self.lib, new_lib);
            self.ext = new_ext;
            self.last_modified = last_modified;

            // Now it's safe to drop the old lib
            drop(old_lib);

            // Delete the old copied file
            let old_path = lib_path.with_extension(old_ext.to_string());
            let _ = fs::remove_file(old_path);
        }
    }
}

impl Drop for HotReloader {
    fn drop(&mut self) {
        let Some(lib_path) = Self::library_path() else { return };
        let current_path = lib_path.with_extension(self.ext.to_string());
        let _ = fs::remove_file(current_path);
    }
}
