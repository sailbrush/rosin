#![no_main]

use std::str::FromStr;
use rosin_core::prelude::Stylesheet;

libfuzzer_sys::fuzz_target!(|data: String| {
    let _ = Stylesheet::from_str(&data);
});
