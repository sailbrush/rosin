
use std::{any::Any, collections::VecDeque, sync::Mutex};

use windows::Win32::Foundation::RECT;
use windows::Win32::Foundation::{HINSTANCE, HWND};
use windows::Win32::Graphics::Direct2D::Common::D2D_SIZE_U;
use windows::Win32::System::Threading::GetCurrentThreadId;
use windows::Win32::UI::WindowsAndMessaging::{GetClientRect, GetDesktopWindow, GetWindowRect, GetWindowThreadProcessId, SW_NORMAL, ShowWindowAsync};
use windows::core::Error;

use crate::{
    kurbo::Size,
    prelude::*,
};

use crate::desc::WindowDesc;

// these 2 are separate to not drop the vec within the same fn,
// to not cause a mem leak and to break apart the "expensive part" and the "cheep part"

/// Converts a utf8 str to a utf16 vector
/// 
/// Allocates and copies the data
pub(crate) fn utf8_to_utf16(string: &str) -> Vec<u16> {
    let os_str = AsRef::<std::ffi::OsStr>::as_ref(string);

    std::os::windows::ffi::OsStrExt::encode_wide(os_str)
        .chain(std::iter::once(0))
        .collect()
}

/// Casts a rust utf16 slice to a win32 utf16 string
/// 
/// Is close to a noop
pub(crate) fn utf16_as_pcwstr(slice: &[u16]) -> windows::core::PCWSTR {
    windows::core::PCWSTR::from_raw(slice.as_ptr())
}

enum ActionBlock {
    Uncalled(u32, Box<dyn FnOnce(&RosinView) -> Box<dyn Any + Send + 'static> + Send + 'static>),
    Called(u32, Box<dyn Any + Send + 'static>),
    Empty,
}

#[allow(unused)]
impl ActionBlock {
    fn is_empty(&self) -> bool {
        matches!(self, ActionBlock::Empty)
    }

    fn is_called(&self) -> bool {
        matches!(self, ActionBlock::Called(_, _))
    }

    fn is_uncalled(&self) -> bool {
        matches!(self, ActionBlock::Uncalled(_, _))
    }
}

/// A struct for safely locking the use of a View on a single thread
pub(crate) struct ThreadLockedView {
    view: RosinView,
    thread_id: u32,

    action_queue: Mutex<VecDeque<Box<dyn FnOnce(&RosinView)>>>,

    // maybe a `Slab` could work better here than a `VecDeque`?
    action_blocks: Mutex<VecDeque<ActionBlock>>,
}

impl ThreadLockedView {
    // this is the best that can be done without knowing the "main" thread from what I've reaserched
    // It's a vary leaky and not-good implementation but windows does not provide the api to know
    // which thread is the "main" thread without acces to the "main" thread.
    //
    // If there is a way to detect the "main" thread then it should be 110% added
    pub fn new(view: RosinView) -> ThreadLockedView {
        const DEFAULT_CAPACITY: usize = 64;

        Self {
            thread_id: unsafe {
                // SAFETY: this is ran on a valid thread
                GetWindowThreadProcessId(view.hwnd(), None)
            },
            view,

            action_queue: Mutex::new(VecDeque::with_capacity(DEFAULT_CAPACITY)),
            action_blocks: Mutex::new(
                VecDeque::from_iter(
                    std::iter::repeat_with(|| ActionBlock::Empty)
                        .take(DEFAULT_CAPACITY)
                )
            ),
        }
    }

    // TODO More testing when able
    /// Tries to execute immidietly
    ///
    /// Returns [`None`] if it's not found on the original thread
    pub fn try_on_thread<F, R>(&self, f: F) -> Option<R>
    where
        F: FnOnce(&RosinView) -> R,
    {
        let current_id = unsafe { GetCurrentThreadId() };
        (self.thread_id == current_id).then(|| f(&self.view))
    }

    /// SAFETY: All actions performed on the given `&RosinView` must be locked to the thread `RosinView` was created on.
    pub unsafe fn try_on_trust<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&RosinView) -> R + Send + 'static,
        R: Send + 'static,
    {
        f(&self.view)
    }

    // TODO More testing when able
    pub fn queue_on_thread<F>(&self, f: F)
    where
        F: FnOnce(&RosinView) + Send + 'static,
    {
        let current_id = unsafe { GetCurrentThreadId() };

        let mut queue = self
            .action_queue
            .lock()
            .expect("Can not do anything if the `action_queue` is poisoned except *maybe* recover it?");

        if self.thread_id == current_id {
            for func in queue.drain(..) {
                func(&self.view)
            }

            std::mem::drop(queue);

            let mut blocks = self
                .action_blocks
                .lock()
                .expect("Can not do anything if the `action_blocks` is poisoned except *maybe* recover it?");

            for action in blocks.iter_mut() {
                if !action.is_uncalled() {
                    continue
                };
                let ActionBlock::Uncalled(thread_id, func) = std::mem::replace(action, ActionBlock::Empty) else {
                    unreachable!("`block_on_thread`: We already sucsesfully checked that action is uncalled")
                };
                *action = ActionBlock::Called(thread_id, func(&self.view))
            }

            f(&self.view)
        } else {
            queue.push_back(Box::new(f))
        }
    }

    // TODO More testing when able
    // could this be done better with async?
    pub fn block_on_thread<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&RosinView) -> R + Send + 'static,
        R: Send + 'static,
    {
        let current_id = unsafe { GetCurrentThreadId() };

        let mut blocks = self
            .action_blocks
            .lock()
            .expect("Can not do anything if the `action_blocks` is poisoned except *maybe* recover it?");

        if self.thread_id == current_id {
            for action in blocks.iter_mut() {
                if !action.is_uncalled() {
                    continue
                };
                let ActionBlock::Uncalled(thread_id, func) = std::mem::replace(action, ActionBlock::Empty) else {
                    unreachable!("`block_on_thread`: We already sucsesfully checked that action is uncalled")
                };
                *action = ActionBlock::Called(thread_id, func(&self.view))
            }

            std::mem::drop(blocks);

            let mut queue = self
                .action_queue
                .lock()
                .expect("Can not do anything if the `action_queue` is poisoned except *maybe* recover it?");

            for func in queue.drain(..) {
                func(&self.view)
            }

            return f(&self.view);
        } else {
            let uncalled_action = ActionBlock::Uncalled(current_id, Box::new(|view| Box::new(f(view))));
            let action = blocks
                .iter_mut()
                .find(|block| block.is_empty());

            match action {
                Some(action) => {
                    *action = uncalled_action;
                }
                None => {
                    blocks.push_back(uncalled_action)
                }
            }

            eprintln!("Warning `block_on_thread`: The block check is likely a vary bad impl that should be replaced");
            loop {
                for block in blocks.iter_mut() {
                    {
                        let ActionBlock::Called(thread_id, _) = block else {
                            continue
                        };

                        if *thread_id != current_id {
                            continue
                        }
                    }

                    let action = std::mem::replace(block, ActionBlock::Empty);

                    let ActionBlock::Called(_, ret) = action else {
                        unreachable!("`block_on_thread`: We already sucsesfully unpacked `block` into a `Action::Called` before")
                    };

                    return *ret.downcast().expect("Boxed the return type `R` earlyer");
                }

                std::thread::sleep(std::time::Duration::from_millis(1))
            }
        }
    }
}

// SAFETY: You can only acces the !Send View and UnsafeCell if it's on the original thread
unsafe impl Send for ThreadLockedView {}

// SAFETY: You can only acces the !Sync View and UnsafeCell if it's on the original thread
unsafe impl Sync for ThreadLockedView {}

pub(crate) struct RosinView {
    hwnd: HWND,
}

// here to easely change the impl later
pub(crate) fn f64_to_i32(f: f64) -> i32 {
    f as i32
}

/// Unsure if this is correct. I highly recommend just deleting this function
#[cfg(false)]
fn menu(desc: &MenuDesc, translation_map: &TranslationMap) -> Result<HMENU, Error> {
    use crate::menu::{
        MenuItem,

        StandardAction,
    };

    // use windows::core::Free;
    use windows::Win32::UI::WindowsAndMessaging::{
        MENUITEMINFOW,
        CreateMenu,
        InsertMenuItemW,
        
        MFT_SEPARATOR,

        MIIM_STRING,
        MIIM_SUBMENU,
    };

    let menu = unsafe {
        // SAFETY: No inputs => all inputs are valid (yipee)
        CreateMenu()?
    };

    // use the InsertMenuItem, AppendMenu, and InsertMenu functions

    for (pos, item) in desc.items.iter().enumerate() {
        let mut title_utf16;

        let mut data = MENUITEMINFOW {
            fMask: MIIM_STRING | MIIM_SUBMENU,
            ..Default::default()
        };
        
        match item {
            MenuItem::Action {
                title,
                command,
                shortcut,
                enabled,
                selected,
            } => {
                title_utf16 = utf8_to_utf16(&title.resolve(translation_map));
                // todo!("adding an action within a menu")
            }

            MenuItem::Submenu { title, menu, enabled } => {
                title_utf16 = utf8_to_utf16(&title.resolve(translation_map));
                data.hSubMenu = self::menu(menu, translation_map)?;
            }

            MenuItem::Standard( standard ) => {
                match standard {
                    StandardAction::Copy      => {
                        title_utf16 = utf8_to_utf16("menu.copy");
                        eprintln!("Only english supported for `Copy` in the menu");
                        // todo!("adding a standard copy action within a menu")
                    }

                    StandardAction::Cut       => {
                        todo!("adding a standard cut action within a menu")
                    }

                    StandardAction::Paste     => {
                        todo!("adding a standard paste action within a menu")
                    }

                    StandardAction::SelectAll => {
                        todo!("adding a standard select all action within a menu")
                    }
                }
            }

            MenuItem::Separator => {
                todo!("adding a separator within the menu")
            }
        };

        data.dwTypeData = windows::core::PWSTR(title_utf16.as_mut_ptr());
        data.cch = title_utf16.len() as u32;

        unsafe {
            InsertMenuItemW(
                menu,
                pos as u32,
                true,
                &raw const data,
            )?
        }
    }

    Ok(menu)
}

// TODO: implement all the unused (menu) stuff
impl RosinView {
    pub fn from_new_window<S: 'static>(
        desc: &WindowDesc<S>,
        instance: Option<HINSTANCE>,
        parent: Option<WindowHandle>,
        _translation_map: &TranslationMap
    ) -> Result<RosinView, Error> {
        use windows::Win32::UI::WindowsAndMessaging::{
            WS_EX_OVERLAPPEDWINDOW,
            WS_CAPTION,
            WS_SYSMENU,
            WS_SIZEBOX,
            WS_MINIMIZEBOX,
            WS_MAXIMIZEBOX,
        };
        
        println!("Initializing rosin view");

        let desktop = unsafe { GetDesktopWindow() };

        let width = f64_to_i32(desc.size.width);
        let height = f64_to_i32(desc.size.height);

        let (x, y) = 'pos: {
            if let Some(pos) = desc.position {
                break 'pos (f64_to_i32(pos.x), f64_to_i32(pos.y));
            }

            let desktop_size = {
                let mut rect = Default::default();
                unsafe {
                    // SAFETY:
                    //  - desktop is a valid window handle
                    //  - &raw mut rect is a valid RECT address
                    GetWindowRect(desktop, &raw mut rect)?;
                }
                rect
            };

            let desktop_width = desktop_size.right - desktop_size.left;
            let desktop_height = desktop_size.bottom - desktop_size.top;

            let x = (desktop_width - width) / 2;
            let y = (desktop_height - height) / 2;

            (x, y)
        };

        let view_state = Box::new(ViewState::new(desc.min_size, desc.max_size));

        #[cfg(false)]
        let menu = desc
            .menu
            .as_ref()
            .map(|desc| menu(desc, translation_map))
            .transpose()?;

        let window_style = {
            let mut window_style = WS_CAPTION | WS_SYSMENU;

            if !desc.close_button {
                todo!("removing the (unsuported officially) close button")
            }

            if desc.maximize_button {
                window_style = window_style | WS_MAXIMIZEBOX;
            }

            if desc.minimize_button {
                window_style = window_style | WS_MINIMIZEBOX;
            }

            if desc.resizeable {
                window_style = window_style | WS_SIZEBOX;
            }

            window_style
        };

        println!("State Pointer Adress: {view_state:p}");

        // I tried looking for another safe
        // or at least safer api for creating a window,
        // but for now this should hopefully do.
        let view = RosinView {
            hwnd: unsafe {
                // FIXME add safety comments
                windows::Win32::UI::WindowsAndMessaging::CreateWindowExW(
                    WS_EX_OVERLAPPEDWINDOW,
                    crate::platform::proc_fn::ROSIN_CLASS,
                    // TODO check: From my testing this "clones" the string; Not 100% sure if it's sound or UB though
                    desc.title
                        .as_deref()
                        .map(utf8_to_utf16)
                        .as_deref()
                        .map(utf16_as_pcwstr)
                        .as_ref(),
                    window_style,
                    x,
                    y,
                    width,
                    height,
                    Some(parent.map(|handle| handle.0.view.view.hwnd).unwrap_or(desktop)),
                    None, // menu,
                    instance,
                    // NOTE: This is given to the actual handle only *after* WM_CREATE or WM_NCCREATE is processed
                    //       (I don't remember which, they are both processed early on but neather
                    //       is guaranteed to be processed before anything else, and from testing they are not)
                    Some(Box::leak(view_state) as *mut _ as *const _),
                )?
            },
        };

        unsafe {
            // SAFETY:
            //  - view.hwnd is a valid handle
            //  - SW_NORMAL is a valid show window code
            ShowWindowAsync(view.hwnd, SW_NORMAL).ok()?;
        }

        Ok(view)
    }

    /// Guaranteed to give a valid handle
    pub fn hwnd(&self) -> HWND {
        self.hwnd
    }

    /// Guaranteed to give a valid ViewState when called
    pub fn get_view_state(&self) -> Option<std::ptr::NonNull<ViewState>> {
        unsafe {
            // SAFETY: self.hwnd() is guaranteed to give a valid handle
            get_view_state(self.hwnd())
        }
    }
}

impl Drop for RosinView {
    fn drop(&mut self) {
        unsafe {
            use windows::Win32::UI::WindowsAndMessaging::{GWLP_USERDATA, GetWindowLongPtrW};

            // SAFETY: self.hwnd is a valid handle
            debug_assert!(!self.hwnd.is_invalid(), "`hwnd` at this point should be a valid window handle");
            if let Some(view_state) = std::ptr::NonNull::new(GetWindowLongPtrW(self.hwnd, GWLP_USERDATA) as *mut ViewState) {
                // SAFETY: view_state is initialized to be valid
                std::mem::drop(Box::from_raw(view_state.as_ptr()));
            }
        }
    }
}


#[derive(Clone, Copy)]
pub(crate) struct ViewStateSize {
    pub x: i32,
    pub y: i32,
}

impl ViewStateSize {
    pub fn from_size(size: Size) -> Self {
        ViewStateSize {
            x: f64_to_i32(size.width),
            y: f64_to_i32(size.height),
        }
    }
    
    pub fn default_max() -> Self {
        // I did not verify if these are the actual values.
        ViewStateSize {
            x: i32::MAX,
            y: i32::MAX,
        }
    }

    pub fn default_min() -> Self {
        ViewStateSize {
            // **I DID NOT VERIFY IF THIS IS THE ACTUAL DEFAULT VALUE.**
            x: 32,
            y: 0,
        }
    }
}

pub(crate) struct ViewStateSizeBounds {
    pub min: ViewStateSize,
    pub max: ViewStateSize,
}

use windows::Win32::Graphics::Direct2D::{D2D1_FACTORY_TYPE_MULTI_THREADED, D2D1CreateFactory, ID2D1Factory8, ID2D1HwndRenderTarget};

#[repr(C)]
pub(crate) struct ViewState {
    pub factory: ID2D1Factory8,
    pub render_target: Option<ID2D1HwndRenderTarget>,
    pub size_bounds: ViewStateSizeBounds,
}

impl ViewState {
    fn new(min_size: Option<Size>, max_size: Option<Size>) -> Result<Self, Error> {
        let factory = unsafe {
            // SAFETY: all inputs are valid
            D2D1CreateFactory(D2D1_FACTORY_TYPE_MULTI_THREADED, None)?
        };

        let size_bounds = ViewStateSizeBounds {
            min: min_size.map(ViewStateSize::from_size).unwrap_or_else(ViewStateSize::default_min),
            max: max_size.map(ViewStateSize::from_size).unwrap_or_else(ViewStateSize::default_max),
        };

        Ok(
            ViewState {
                factory,
                render_target: None,
                size_bounds: size_bounds,
            }
        )
    }

    /// Initalizes all the state
    ///
    /// SAFETY: `hwnd` must be a valid handle
    #[cfg(false)]
    #[allow(unsafe_op_in_unsafe_fn)]
    pub unsafe fn init(&mut self, hwnd: HWND) -> Result<(), Error> {
        debug_assert!(!hwnd.is_invalid(), "`hwnd` at this point should be a valid window handle");
        self.init_graphics(hwnd)?;

        Ok(())
    }

    /// Releases all the state
    pub fn release(&mut self) {
        self.release_graphics();
    }

    /// Initializes the graphics API
    ///
    /// SAFETY: `hwnd` must be a valid handle
    pub unsafe fn init_graphics(&mut self, hwnd: HWND) -> Result<(), Error> {
        if self.render_target.is_none() {
            debug_assert!(!hwnd.is_invalid(), "`hwnd` at this point should be a valid window handle");

            let rect = unsafe {
                let mut rect = RECT::default();
                // SAFETY:
                //  - hwnd is a valid handle
                //  - &raw mut rect is pointing to a valid memory addres
                GetClientRect(hwnd, &raw mut rect)?;
                rect
            };

            let render_target_properties = Default::default();
            let hwnd_render_target_properties = windows::Win32::Graphics::Direct2D::D2D1_HWND_RENDER_TARGET_PROPERTIES {
                hwnd,
                pixelSize: size_of_rect(rect),
                presentOptions: Default::default(),
            };

            let render_target = unsafe {
                // SAFETY: All these poitners point to valid values
                self.factory
                    .CreateHwndRenderTarget(&raw const render_target_properties, &raw const hwnd_render_target_properties)?
            };

            let _ = self.render_target.insert(render_target);
        }

        Ok(())
    }

    /// Releases the graphics API
    pub fn release_graphics(&mut self) {
        self.render_target = None;
    }
}

impl Drop for ViewState {
    fn drop(&mut self) {
        self.release()
    }
}

fn size_of_rect(rect: RECT) -> D2D_SIZE_U {
    D2D_SIZE_U {
        width: (rect.right - rect.left) as u32,
        height: i32::abs(rect.top - rect.bottom) as u32,
    }
}

/// SAFETY: hwnd must be a handle to a valid window
pub unsafe fn get_view_state(hwnd: HWND) -> Option<std::ptr::NonNull<ViewState>> {
    use windows::Win32::UI::WindowsAndMessaging::{
        GetWindowLongPtrW,
        GWLP_USERDATA,
    };

    unsafe {
        // SAFETY: hwnd is valid
        std::ptr::NonNull::new(GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut ViewState)
    }
}
