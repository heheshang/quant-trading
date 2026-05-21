use tokio_tungstenite::{connect_async, tungstenite::Message};
use url::Url;

#[derive(Debug)]
pub struct WsStressClient {
    uri: String,
}

impl WsStressClient {
    pub async fn new(uri: &str) -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self {
            uri: uri.to_string(),
        })
    }

    pub async fn connect(&self) -> Result<(), Box<dyn std::error::Error>> {
        let (_ws_stream, _) = connect_async(&self.uri).await?;
        Ok(())
    }
}

impl Clone for WsStressClient {
    fn clone(&self) -> Self {
        Self {
            uri: self.uri.clone(),
        }
    }
}
