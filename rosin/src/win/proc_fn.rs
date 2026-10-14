use std::ptr::NonNull;

use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows::Win32::Graphics::Gdi::{BeginPaint, COLOR_WINDOW, EndPaint, FillRect, HBRUSH, HDC, /* InvalidateRect, */ PAINTSTRUCT};
use windows::Win32::UI::WindowsAndMessaging::{DefWindowProcW, PostQuitMessage, MINMAXINFO, WM_DESTROY, WM_NCCREATE, WM_PAINT, /* WM_SIZE, */ WM_GETMINMAXINFO};
use windows::core::{Error, PCWSTR, w};

use crate::platform::view::{ViewState, get_view_state};

/// The main class of all windows on windows_os in rosin
pub(crate) const ROSIN_CLASS: PCWSTR = w!("RosinGUI Windows Class");
const OK: LRESULT = LRESULT(0);

/// The main procedrual fuction for handling messages in rosin on windows
pub(crate) unsafe extern "system" fn proc(hwnd: HWND, msg: u32, w_param: WPARAM, l_param: LPARAM) -> LRESULT {
    let view_state = match msg {
        WM_NCCREATE => {
            return unsafe {
                use windows::Win32::UI::WindowsAndMessaging::{
                    CREATESTRUCTW,
                    SetWindowLongPtrW,
                    GWLP_USERDATA,
                };
                #[cfg(debug_assertions)]
                println!("Creating window `{hwnd:?}`");

                // Guaranteed to get a CREATESTRUCT
                let create_struct = (l_param.0 as *const CREATESTRUCTW).as_ref().unwrap_unchecked();

                // SAFETY: parameters are given as is => they are all valid
                SetWindowLongPtrW(hwnd, GWLP_USERDATA, create_struct.lpCreateParams as isize);

                // SAFETY: parameters are given as is => they are all valid
                DefWindowProcW(hwnd, msg, w_param, l_param)
            };
        }
 
        _ => if cfg!(debug_assertions) && hwnd.is_invalid() {
            eprintln!("window handle `{hwnd:?}` is invalid!");
            None
        } else {
            unsafe {
                // SAFETY:
                //  - hwnd is a valid handle
                get_view_state(hwnd)
            }
        },
    };

    match msg {
        WM_PAINT => {
            let mut paint_struct = PAINTSTRUCT::default();

            let hdc = unsafe {
                // SAFETY: all given inputs point to valid data
                BeginPaint(hwnd, &raw mut paint_struct)
            };

            let result = if let Some(state) = view_state {
                unsafe {
                    // SAFETY: all given inputs point to valid data
                    paint(hwnd, hdc, paint_struct, state)
                }
            } else {
                Ok(())
            };

            unsafe {
                // SAFETY: all given inputs point to valid data
                let _ = EndPaint(hwnd, &raw mut paint_struct);
            }

            match result {
                Ok(()) => OK,
                Err(_) => LRESULT(-1),
            }
        }
        #[cfg(false)] 
        WM_SIZE => {
            let Some(view_state) = view_state else {
                return unsafe {
                    // SAFETY: parameters are given as is => they are all valid
                    DefWindowProcW(hwnd, msg, w_param, l_param)
                }
            };

            let resize = match w_param.0.max(10) as u32 {
                windows::Win32::UI::WindowsAndMessaging::SIZE_RESTORED => Resize::Restore(0, 0),
                windows::Win32::UI::WindowsAndMessaging::SIZE_MINIMIZED => Resize::Maximize,
                windows::Win32::UI::WindowsAndMessaging::SIZE_MAXIMIZED => Resize::Maximize,
                windows::Win32::UI::WindowsAndMessaging::SIZE_MAXHIDE => Resize::MaxHide,
                windows::Win32::UI::WindowsAndMessaging::SIZE_MAXSHOW => Resize::MaxShow(0, 0),
                _ => unreachable!("`w_param` should only have a value in `0..5`."),
            };

            println!("{resize:?}");

            unsafe {
                // SAFETY:
                //  - hwnd is a valid handle
                //  - view_state is initialized as a valid ViewState
                match self::resize(hwnd, resize, view_state) {
                    Ok(()) => OK,
                    Err(_) => LRESULT(-1),
                }
            }
        }
        WM_GETMINMAXINFO => {
            let Some(view_state) = view_state else {
                return OK;
            };

            let minmaxinfo = unsafe {
                // SAFETY: l_param is guaranteed to be a pointer to MINMAXINFO
                NonNull::new_unchecked(l_param.0 as *mut MINMAXINFO).as_mut()
            };

            let view_state = unsafe {
                // SAFETY: View state is initialized
                view_state.as_ref()
            };

            #[cfg(debug_assertions)]
            println!(
                "CURRENT MIN/MAX SIZE:\n\tmax.x = {max_x}\n\tmax.y = {max_y}\n\tmin.x = {min_x}\n\tmin.y = {min_y}",
                max_x = minmaxinfo.ptMaxTrackSize.x,
                max_y = minmaxinfo.ptMaxTrackSize.y,
                min_y = minmaxinfo.ptMinTrackSize.y,
                min_x = minmaxinfo.ptMinTrackSize.x,
            );

            minmaxinfo.ptMaxTrackSize.x = view_state.size_bounds.max.x;
            minmaxinfo.ptMaxTrackSize.y = view_state.size_bounds.max.y;
            minmaxinfo.ptMinTrackSize.y = view_state.size_bounds.min.y;
            minmaxinfo.ptMinTrackSize.x = view_state.size_bounds.min.x;

            OK
        }
        WM_DESTROY => {
            unsafe {
                // SAFETY: the window is getting distroyed here
                // UNSAFETY(?): Other windows are ignored
                PostQuitMessage(0);
                return OK;
            }
        }
        _ => unsafe {
            // SAFETY: parameters are given as is => they are all valid
            DefWindowProcW(hwnd, msg, w_param, l_param)
        },
    }
}

/// Ment to run when paining a portion of a window (usually on a WM_PAINT message)
///
/// SAFETY:
///  - `hwnd` must be a valid handle
///  - `hdc` must be from the `hwnd` handle
///  - `paint_struct` must be from the `hwnd` handle
///  - `state` must point to the memory addres of the `hwnd`s state
///  - `state` must be initialized
#[allow(unsafe_op_in_unsafe_fn)]
unsafe fn paint(hwnd: HWND, hdc: HDC, mut paint_struct: PAINTSTRUCT, mut state: NonNull<ViewState>) -> Result<(), Error> {
    debug_assert!(!hwnd.is_invalid(), "`hwnd` at this point should be a valid window handle");

    let _ = FillRect(hdc, &raw mut paint_struct.rcPaint, HBRUSH(COLOR_WINDOW.0 as *mut _));

    // SAFETY:
    //  - `as_mut`: state is a valid ViewState
    //  - `init_graphics`: hwnd is a valid handle
    state.as_mut().init_graphics(hwnd)?;

    Ok(())
}

/*

/// Helper enum for resizing a window
#[repr(u32)]
#[derive(Debug)]
enum Resize {
    /// This window is restored
    Restore(usize, usize) = windows::Win32::UI::WindowsAndMessaging::SIZE_RESTORED,

    /// This window is minimized
    Minimize = windows::Win32::UI::WindowsAndMessaging::SIZE_MINIMIZED,

    /// This window is maximized
    Maximize = windows::Win32::UI::WindowsAndMessaging::SIZE_MAXIMIZED,

    /// Hide due to another window maximizing
    MaxHide = windows::Win32::UI::WindowsAndMessaging::SIZE_MAXHIDE,

    /// Show due to another window being restored/minimized
    MaxShow(usize, usize) = windows::Win32::UI::WindowsAndMessaging::SIZE_MAXSHOW,
}

/// Ment to run when a window is resized (aka on a WM_SIZE message or simmilar)
///
/// SAFETY:
///  - `hwnd` must be a valid handle
///  - `state` must point to the memory addres of the `hwnd`s state
///  - `state` must be initialized
#[allow(unsafe_op_in_unsafe_fn)]
unsafe fn resize(hwnd: HWND, _resize: Resize, state: NonNull<ViewState>) -> Result<(), windows::core::Error> {
    debug_assert!(!hwnd.is_invalid(), "`hwnd` at this point should be a valid window handle");

    use windows::Win32::Graphics::Direct2D::Common::D2D_SIZE_U;
    use windows::Win32::UI::WindowsAndMessaging::GetClientRect;

    // SAFETY: state is a valid ViewState
    if let Some(render_target) = unsafe { &state.as_ref().render_target } {
        let mut rc = Default::default();
        // SAFETY:
        //  - hwnd is a valid handle
        //  - hwnd is a valid handle
        GetClientRect(hwnd, &raw mut rc)?;

        let size = D2D_SIZE_U {
            width: rc.right as u32,
            height: rc.bottom as u32,
        };

        unsafe {
            // SAFETY: &raw const size is a valid pointer to a D2D_SIZE_U
            render_target.Resize(&raw const size)?;
            // calculate_layout(render_target);
        }

        // SAFETY:
        //  - hwnd is a valid window handle
        //  - all other inputs are valid
        InvalidateRect(Some(hwnd), None, false).ok()?;
    }

    Ok(())
}

*/
