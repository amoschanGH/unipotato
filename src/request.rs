use hyper::body::Incoming;
use http_body_util::BodyExt;
use std::collections::HashMap;
use serde::de::DeserializeOwned;

pub struct Query(HashMap<String, String>);

impl Query {
    pub fn from_uri(uri: &hyper::Uri) -> Self {
        let mut params = HashMap::new();
        if let Some(query) = uri.query() {
            for pair in query.split('&') {
                if let Some((key, value)) = pair.split_once('=') {
                    params.insert(
                        urlencoding::decode(key).unwrap_or_default().to_string(),
                        urlencoding::decode(value).unwrap_or_default().to_string(),
                    );
                }
            }
        }
        Query(params)
    }

    pub fn get(&self, key: &str) -> Option<&String> {
        self.0.get(key)
    }

    pub fn get_or(&self, key: &str, default: &str) -> String {
        self.0.get(key).cloned().unwrap_or_else(|| default.to_string())
    }
}

pub struct Body {
    data: Vec<u8>,
}

impl Body {
    pub async fn from_incoming(incoming: Incoming) -> Result<Self, String> {
        let collected = incoming
            .collect()
            .await
            .map_err(|e| format!("Failed to read body: {}", e))?;
        
        let data = collected.to_bytes().to_vec();
        Ok(Body { data })
    }

    pub fn as_str(&self) -> Result<&str, String> {
        std::str::from_utf8(&self.data)
            .map_err(|e| format!("Invalid UTF-8: {}", e))
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.data
    }

    pub fn json<T: DeserializeOwned>(&self) -> Result<T, String> {
        let text = self.as_str()?;
        serde_json::from_str(text)
            .map_err(|e| format!("Failed to parse JSON: {}", e))
    }

    pub fn form(&self) -> Result<HashMap<String, String>, String> {
        let text = self.as_str()?;
        let mut params = HashMap::new();
        for pair in text.split('&') {
            if let Some((key, value)) = pair.split_once('=') {
                params.insert(
                    urlencoding::decode(key).unwrap_or_default().to_string(),
                    urlencoding::decode(value).unwrap_or_default().to_string(),
                );
            }
        }
        Ok(params)
    }
}

// Helper functions to extract query and body from request
pub async fn extract_query(req: &hyper::Request<Incoming>) -> Query {
    Query::from_uri(req.uri())
}

pub async fn extract_body(req: hyper::Request<Incoming>) -> Result<(hyper::Request<()>, Body), String> {
    let (parts, body) = req.into_parts();
    let body = Body::from_incoming(body).await?;
    let req = hyper::Request::from_parts(parts, ());
    Ok((req, body))
}
