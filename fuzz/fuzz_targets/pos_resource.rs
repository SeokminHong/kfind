#![no_main]

use kfind_data::decode_pos_lexicon;
use libfuzzer_sys::fuzz_target;

mod seed;

fuzz_target!(|data: &[u8]| {
    let data = seed::decode_hex(data);
    let Ok(resource) = decode_pos_lexicon(&data) else {
        return;
    };

    let _ = resource.stats();
    for entry in resource.entries().iter().take(16) {
        let _ = resource.lookup_fine_pos(&entry.lemma).count();
    }
});
