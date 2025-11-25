use crate::platform::{
    http_client::WHATWGFetchHttpClient,
    singlethreaded::{apc::PassingContext, transferables::FlatTransferables},
};

pub mod apc;
pub mod transferables;
#[cfg(target_arch = "wasm32")]
pub mod wasm_entries;

#[cfg(not(target_arch = "wasm32"))]
pub mod wasm_entries {
    use wasm_bindgen::prelude::*;
    #[wasm_bindgen]
    pub fn singlethreaded_receive_data() {}
    // Stub other functions if necessary, or just let the usage site fail if they are used on host?
    // wasm_entries functions are usually called by JS, not by other Rust code.
}

pub type UsedTransferables = FlatTransferables;
pub type UsedHttpClient = WHATWGFetchHttpClient;
pub type UsedContext = PassingContext;
