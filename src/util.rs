use std::{
    collections::BTreeMap,
    sync::Mutex,
};

pub struct Context {
    pub name: String,
    pub domain: String,
    pub path: String,
    pub src: String,
    pub proxy: Mutex<BTreeMap<String, String>>,
}
