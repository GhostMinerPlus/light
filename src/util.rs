use std::{sync::Mutex, collections::BTreeMap, io::{Error, self}};
use pnet::datalink;

pub struct Context {
    pub name: String,
    pub domain: String,
    pub path: String,
    pub src: String,
    pub proxy: Mutex<BTreeMap<String, String>>,
}

pub fn get_global_ipv6() -> io::Result<String> {
    let interfaces = datalink::interfaces();
    for interface in &interfaces {
        for ip in &interface.ips {
            if ip.is_ipv6() {
                let ip_s = ip.ip().to_string();
                if !ip_s.starts_with("f") && !ip_s.starts_with(":") {
                    return Ok(ip_s);
                }
            }
        }
    }

    Err(Error::new(io::ErrorKind::Other, ""))
}

#[cfg(test)]
mod tests {
    #[test]
    fn test() {
        let ipv6 = super::get_global_ipv6().unwrap();
        println!("{ipv6}");
    }
}
