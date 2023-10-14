use std::collections::BTreeMap;

// public
#[derive(serde::Deserialize, serde::Serialize, earth::Config)]
pub struct Config {
    pub name: String,
    pub domain: String,
    pub path: String,
    pub hosts: Vec<String>,
    pub proxy: BTreeMap<String, String>,
    pub log_level: String,
    pub src: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            name: "light".to_string(),
            domain: "[::]:8080".to_string(),
            path: "/light".to_string(),
            hosts: Vec::new(),
            proxy: BTreeMap::new(),
            log_level: "info".to_string(),
            src: format!("dist"),
        }
    }
}
