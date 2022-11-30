use crate::types::Client;
use anyhow::{anyhow, Result};
use geohash::{encode, Coordinate};
use serde_derive::Deserialize;

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct IpInfo {
    pub status: String,
    pub query: String,
    #[serde(default)]
    pub message: String,
    #[serde(default)]
    pub lat: f64,
    #[serde(default)]
    pub lon: f64,
    #[serde(default)]
    pub country_code: String,
    #[serde(default)]
    pub timezone: String,
    #[serde(default)]
    pub isp: String,
    #[serde(default)]
    pub r#as: String,
    /// geohash is not provided by the API, but we can calculate and insert it
    #[serde(default)]
    pub geo: String,
}

/// Get the public ip info from ip_api.com (45 req/hr limit)
// todo: find a suitable API with no rate limits and commercial use
pub async fn get_ip_info(addr: Option<String>) -> Result<IpInfo> {
    let addr = addr.map(|ip| format!("/{}", ip)).unwrap_or_default();
    let url = format!("http://ip-api.com/json{}", addr);

    let res = Client::new().get(url.parse()?).await?;
    let data = hyper::body::to_bytes(res.into_body()).await?;

    let mut info: IpInfo = serde_json::from_slice(&data)?;

    if info.status == "success" {
        info.geo = geohash(info.lat, info.lon)?;
        Ok(info)
    } else {
        Err(anyhow!(
            "{}: {} - {}",
            info.status,
            info.query,
            info.message
        ))
    }
}

pub fn geohash(lat: f64, lon: f64) -> Result<String> {
    let coord = Coordinate { x: lon, y: lat };
    encode(coord, 7).map_err(|e| anyhow!(e))
}

mod tests {
    use super::{geohash, get_ip_info};

    #[tokio::test]
    async fn test_geohash() {
        let hash = geohash(0.0, 0.0).unwrap();
        assert_eq!(hash, "s000000");
    }

    #[tokio::test]
    async fn test_self_ip_info() {
        let info = get_ip_info(None).await.unwrap();
        assert_eq!(info.status, "success");
    }

    #[tokio::test]
    async fn test_remote_ip_info() {
        let info = get_ip_info(Some("8.8.8.8".to_string())).await.unwrap();
        assert_eq!(info.status, "success");
    }

    #[tokio::test]
    async fn test_dns_info() {
        let info = get_ip_info(Some("google.com".into())).await.unwrap();
        assert_eq!(info.status, "success");
    }
}
