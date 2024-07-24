use std::io;

use edge_lib::ScriptTree;

pub mod native {
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
}

pub async fn http_execute1(uri: &str, script_tree: &ScriptTree) -> io::Result<String> {
    let res = reqwest::Client::new()
        .post(format!("{uri}/execute1"))
        .header("Content-Type", "application/json")
        .body(serde_json::to_string(script_tree).unwrap())
        .send()
        .await
        .map_err(|e| {
            log::error!("{e}");
            io::Error::other(e)
        })?;
    res.text().await.map_err(|e| {
        log::error!("{e}");
        io::Error::other(e)
    })
}

pub fn gen_token(key: &str, email: String, life_op: Option<u64>) -> io::Result<String> {
    main::gen_token(key, email, life_op)
}

mod main {
    use std::{collections::BTreeMap, io, time};

    use hmac::{digest::KeyInit, Hmac};
    use jwt::{AlgorithmType, Header, SignWithKey, Token};
    use sha2::Sha512;

    pub fn gen_token(key: &str, email: String, life_op: Option<u64>) -> io::Result<String> {
        let key: Hmac<Sha512> =
            Hmac::new_from_slice(&hex2byte_v(key)).map_err(|e| io::Error::other(e))?;
        let header = Header {
            algorithm: AlgorithmType::Hs512,
            ..Default::default()
        };
        let mut claims = BTreeMap::new();
        if let Some(life) = life_op {
            let exp = time::SystemTime::now()
                .duration_since(time::UNIX_EPOCH)
                .expect("can not get timestamp")
                .as_secs()
                + life;
            claims.insert("exp", format!("{exp}"));
        }
        claims.insert("email", email);
        Ok(Token::new(header, claims)
            .sign_with_key(&key)
            .map_err(|e| io::Error::other(e))?
            .as_str()
            .to_string())
    }

    fn hex2byte_v(s: &str) -> Vec<u8> {
        let mut byte_v = Vec::with_capacity(s.len() / 2 + 1);
        let mut is_h = true;
        for ch in s.to_lowercase().chars() {
            if is_h {
                is_h = false;
                let v = if ch >= '0' && ch <= '9' {
                    (ch as u32 - '0' as u32) as u8
                } else {
                    (ch as u32 - 'a' as u32) as u8 + 10
                };
                byte_v.push(v);
            } else {
                is_h = true;
                let v = if ch >= '0' && ch <= '9' {
                    (ch as u32 - '0' as u32) as u8
                } else {
                    (ch as u32 - 'a' as u32) as u8 + 10
                };
                *byte_v.last_mut().unwrap() <<= 4;
                *byte_v.last_mut().unwrap() |= v;
            }
        }
        byte_v
    }
}
