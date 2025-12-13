//! HTTP client for communicating with the tt-ctl controller.

use ruc::*;
use serde::de::DeserializeOwned;
use serde::Serialize;
use ttcore::api::ApiResp;

/// Controller API client.
pub struct Client {
    base_url: String,
    http: reqwest::Client,
}

impl Client {
    pub fn new(addr: &str) -> Self {
        let base_url = if addr.starts_with("http") {
            addr.to_string()
        } else {
            format!("http://{addr}")
        };

        Self {
            base_url,
            http: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(60))
                .build()
                .unwrap(),
        }
    }

    /// GET request, returning deserialized data.
    pub async fn get<T: DeserializeOwned>(&self, path: &str) -> Result<T> {
        let url = format!("{}{path}", self.base_url);
        let resp = self.http.get(&url).send().await.c(d!("request failed"))?;
        let status = resp.status();
        let body: ApiResp<T> = resp.json().await.c(d!("invalid response"))?;

        if body.ok {
            body.data.ok_or_else(|| eg!("empty response"))
        } else {
            Err(eg!(body.error.unwrap_or_else(|| format!("HTTP {status}"))))
        }
    }

    /// POST request with JSON body, returning deserialized data.
    pub async fn post<B: Serialize, T: DeserializeOwned>(
        &self,
        path: &str,
        body: &B,
    ) -> Result<T> {
        let url = format!("{}{path}", self.base_url);
        let resp = self
            .http
            .post(&url)
            .json(body)
            .send()
            .await
            .c(d!("request failed"))?;
        let status = resp.status();
        let body: ApiResp<T> = resp.json().await.c(d!("invalid response"))?;

        if body.ok {
            body.data.ok_or_else(|| eg!("empty response"))
        } else {
            Err(eg!(body.error.unwrap_or_else(|| format!("HTTP {status}"))))
        }
    }

    /// POST request with no request body, no response body.
    pub async fn post_action(&self, path: &str) -> Result<()> {
        let url = format!("{}{path}", self.base_url);
        let resp = self.http.post(&url).send().await.c(d!("request failed"))?;
        let status = resp.status();
        let body: ApiResp<()> = resp.json().await.c(d!("invalid response"))?;

        if body.ok {
            Ok(())
        } else {
            Err(eg!(body.error.unwrap_or_else(|| format!("HTTP {status}"))))
        }
    }

    /// DELETE request.
    pub async fn delete(&self, path: &str) -> Result<()> {
        let url = format!("{}{path}", self.base_url);
        let resp = self.http.delete(&url).send().await.c(d!("request failed"))?;
        let status = resp.status();
        let body: ApiResp<()> = resp.json().await.c(d!("invalid response"))?;

        if body.ok {
            Ok(())
        } else {
            Err(eg!(body.error.unwrap_or_else(|| format!("HTTP {status}"))))
        }
    }
}

// ── Configuration File ──────────────────────────────────────────────

const CONFIG_FILE: &str = ".ttconfig";

/// Read the controller address from ~/.ttconfig.
pub fn load_config() -> Option<String> {
    let path = dirs_path();
    std::fs::read_to_string(&path).ok().map(|s| s.trim().to_string())
}

/// Save the controller address to ~/.ttconfig.
pub fn save_config(addr: &str) -> Result<()> {
    let path = dirs_path();
    std::fs::write(&path, addr).c(d!("save config"))
}

fn dirs_path() -> String {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
    format!("{home}/{CONFIG_FILE}")
}
