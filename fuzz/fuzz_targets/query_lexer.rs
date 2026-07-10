#![no_main]

use kfind_query::{CompileOptions, parse_query};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let source = String::from_utf8_lossy(data);
    let _ = parse_query(&source, &CompileOptions::default());
});
