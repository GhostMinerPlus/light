//! Let light be able to serve.

pub mod connector;
pub mod server;

mod native {
    use pnet::datalink;
    use std::io;

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

        Err(io::Error::new(
            io::ErrorKind::NotFound,
            "Faild to get a global ipv6",
        ))
    }

    pub async fn http_execute_script(uri: &str, script: &[String]) -> io::Result<Vec<String>> {
        let res = reqwest::Client::new()
            .post(format!("{uri}/execute"))
            .header("Content-Type", "application/json")
            .body(serde_json::to_string(script).unwrap())
            .send()
            .await
            .map_err(|e| {
                log::error!("{e}");
                io::Error::other(e)
            })?;
        Ok(serde_json::from_str(&res.text().await.map_err(|e| {
            log::error!("{e}");
            io::Error::other(e)
        })?)
        .unwrap())
    }
}
