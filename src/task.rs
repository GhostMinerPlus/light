use std::time::Duration;

use crate::util;

pub async fn report_address6(name: &str, port: u16, host_v: &Vec<String>) {
    match util::get_global_ipv6() {
        Ok(ipv6) => {
            let data = format!("{{\"name\":\"{}\",\"address\":\"[{ipv6}]:{port}\"}}", name);
            for host in host_v {
                let host = host.clone();
                let data = data.clone();
                log::info!("reporting ip to {host}");
                tokio::spawn(async move {
                    match reqwest::Client::new()
                        .post(&host)
                        .header("Content-Type", "application/json")
                        .body(data)
                        .send()
                        .await
                    {
                        Ok(_) => log::info!("reported ip to {host}"),
                        Err(e) => log::error!("failed to report ip to {host}: {e}"),
                    }
                });
            }
            tokio::time::sleep(Duration::from_secs(60 * 5)).await;
        }
        Err(e) => {
            log::error!("failed to get ip: {e}");
            tokio::time::sleep(Duration::from_secs(60)).await;
        }
    }
}
