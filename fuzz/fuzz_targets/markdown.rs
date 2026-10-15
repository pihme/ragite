#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let text = String::from_utf8_lossy(data);
    if let Err(err) = ragite_core::check_markdown(&text) {
        panic!("{err}");
    }
    if let Err(err) = ragite_core::check_plain(&text) {
        panic!("{err}");
    }
});
