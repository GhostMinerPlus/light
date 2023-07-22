use serde::{self, Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
pub struct Proxy {
    pub path: String,
    pub url: String,
}

#[derive(Deserialize)]
pub struct Page {
    pub size: usize,
    pub number: usize,
}
