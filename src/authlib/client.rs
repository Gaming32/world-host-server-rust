use reqwest::Url;
use serde::de::DeserializeOwned;
use std::time::Duration;

pub struct MinecraftClient {
    client: reqwest::Client,
}

impl MinecraftClient {
    pub fn unauthenticated() -> Self {
        let client = reqwest::ClientBuilder::new()
            .connect_timeout(Duration::from_millis(5000))
            .read_timeout(Duration::from_millis(5000))
            .build()
            .unwrap();
        MinecraftClient { client }
    }

    pub async fn get<T: DeserializeOwned>(&self, url: Url) -> anyhow::Result<Option<T>> {
        let response = self.client.get(url).send().await?;
        let status = response.status();
        if status.as_u16() < 400 {
            let result = response.bytes().await?;
            if result.is_empty() {
                return Ok(None);
            }
            Ok(Some(serde_json::from_slice(&result)?))
        } else {
            Ok(None)
        }
    }
}
