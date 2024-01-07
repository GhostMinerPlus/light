use std::{
    collections::BTreeMap,
    sync::{Mutex, Arc},
};

#[derive(Clone)]
pub struct Context {
    pub name: String,
    pub domain: String,
    pub path: String,
    pub src: String,
    pub proxy: Arc<Mutex<BTreeMap<String, String>>>,
}
