use serde::{self, Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
pub(crate) struct Proxy {
    pub(crate) name: String,
    pub(crate) url: String,
}

#[derive(Deserialize)]
pub(crate) struct Page {
    pub(crate) size: usize,
    pub(crate) number: usize,
}
