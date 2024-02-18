use pnet::datalink;
use std::io;

fn get_global_ipv6() -> io::Result<String> {
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
        "Faild to get a global ipv6 address",
    ))
}

// Public
pub async fn report_uri(name: &str, port: u16, path: &str, host_v: &Vec<String>) -> io::Result<()> {
    let ipv6 = get_global_ipv6()?;
    let uri = format!("http://[{ipv6}]:{port}{path}");
    let data = format!("{{\"name\":\"{}\",\"address\":\"{uri}\"}}", name);
    for host in host_v {
        let host = host.clone();
        let data = data.clone();
        tokio::spawn(async move {
            log::info!("reporting uri to {host}");
            match reqwest::Client::new()
                .post(&host)
                .header("Content-Type", "application/json")
                .body(data)
                .send()
                .await
            {
                Ok(_) => log::info!("reported uri to {host}"),
                Err(e) => log::error!("failed to report uri to {host}: {e}"),
            }
        });
    }
    Ok(())
}
