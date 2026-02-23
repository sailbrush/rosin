#![no_main]

use rosin_core::prelude::{langids, TranslationFile};

libfuzzer_sys::fuzz_target!(|data: String| {
    let _ = TranslationFile::from_str(langids!("en-US"), &data);
});
