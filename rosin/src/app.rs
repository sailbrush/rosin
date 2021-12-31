#![forbid(unsafe_code)]

use crate::libloader::LibLoader;
#[cfg(all(debug_assertions, feature = "hot-reload"))]
use crate::libloader::DYLIB_EXT;

use crate::prelude::*;
use crate::style::*;
use crate::window::*;
use crate::app::*;

#[cfg(all(debug_assertions, feature = "hot-reload"))]
use std::{env, path::Path};
use std::{error, fmt::Debug, mem, time::Duration, time::Instant};
use std::any::Any;

use druid_shell::kurbo::{Line, Size};
use druid_shell::piet::{Color, RenderContext};

use druid_shell::{
    Application, Cursor, FileDialogOptions, FileDialogToken, FileInfo, FileSpec, HotKey, KeyEvent,
    Menu, MouseEvent, Region, SysMods, TimerToken, WinHandler, WindowBuilder, WindowHandle,
};

