use crate::types::Client;
use anyhow::{anyhow, Result};
use geohash::{encode, Coordinate};
use serde_derive::Deserialize;

#[derive(Deserialize, Default, Debug, Clone)]
#[serde(rename_all = "camelCase", default)]
pub struct IpInfoResponse {
    pub ip: String,
    pub hostname: String,
    pub city: String,
    pub region: String,
    pub country: String,
    pub loc: String,
    pub org: String,
    pub postal: String,
    pub timezone: String,
    pub geo: String,
}

/// Get public ip info from https://ipinfo.io
pub async fn get_ip_info(token: String, addr: String) -> Result<IpInfoResponse> {
    let url = format!("http://ipinfo.io/{}?{}", addr, token);
    let res = Client::new().get(url.parse()?).await?;
    let data = hyper::body::to_bytes(res.into_body()).await?;
    let mut info: IpInfoResponse = serde_json::from_slice(&data)?;
    let loc = info.loc.split(',').collect::<Vec<&str>>();
    let lat = loc[0].parse::<f64>()?;
    let lon = loc[1].parse::<f64>()?;
    info.geo = geohash(lat, lon)?;
    Ok(info)
}

pub fn geohash(lat: f64, lon: f64) -> Result<String> {
    let coord = Coordinate { x: lon, y: lat };
    encode(coord, 7).map_err(|e| anyhow!(e))
}

mod tests {
    use super::{geohash, get_ip_info};

    fn token() -> String {
        std::env::var("IPINFO_TOKEN").expect("IPINFO_TOKEN is not set")
    }

    #[tokio::test]
    async fn test_geohash() {
        let hash = geohash(0.0, 0.0).unwrap();
        assert_eq!(hash, "s000000");
    }

    #[tokio::test]
    async fn test_self_ip_info() {
        get_ip_info(token(), "".to_string()).await.unwrap();
    }

    #[tokio::test]
    async fn test_remote_ip_info() {
        get_ip_info(token(), "8.8.8.8".to_string()).await.unwrap();
    }

    #[tokio::test]
    async fn test_dns_info() {
        get_ip_info(token(), "google.com".into()).await.unwrap();
    }
}
