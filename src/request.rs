use hyper::body::Incoming;
use http_body_util::BodyExt;
use std::collections::HashMap;
use serde::de::DeserializeOwned;

/// Query parameter parser for URL queries
pub struct Query(HashMap<String, String>);

impl Query {
    /// Parse query parameters from a URI
    /// 
    /// # Example
    /// ```
    /// let query = Query::from_uri(req.uri());
    /// let page = query.get("page");
    /// ```
    pub fn from_uri(uri: &hyper::Uri) -> Self {
        let params = uri.query()
            .map(Self::parse_query_string)
            .unwrap_or_default();
        
        Query(params)
    }

    /// Get a query parameter value
    pub fn get(&self, key: &str) -> Option<&String> {
        self.0.get(key)
    }

    /// Get a query parameter with a default value
    pub fn get_or(&self, key: &str, default: &str) -> String {
        self.0.get(key)
            .cloned()
            .unwrap_or_else(|| default.to_string())
    }

    /// Check if a query parameter exists
    pub fn has(&self, key: &str) -> bool {
        self.0.contains_key(key)
    }

    /// Get all query parameters
    pub fn all(&self) -> &HashMap<String, String> {
        &self.0
    }

    // === Private Helpers ===

    fn parse_query_string(query: &str) -> HashMap<String, String> {
        query.split('&')
            .filter_map(Self::parse_key_value_pair)
            .collect()
    }

    fn parse_key_value_pair(pair: &str) -> Option<(String, String)> {
        pair.split_once('=').map(|(key, value)| {
            (
                Self::decode_param(key),
                Self::decode_param(value),
            )
        })
    }

    fn decode_param(s: &str) -> String {
        urlencoding::decode(s)
            .unwrap_or_default()
            .to_string()
    }
}

/// HTTP request body parser
pub struct Body {
    data: Vec<u8>,
}

impl Body {
    /// Read body from incoming request
    pub async fn from_incoming(incoming: Incoming) -> Result<Self, String> {
        let collected = incoming.collect().await
            .map_err(|e| format!("Failed to read body: {}", e))?;
        
        let data = collected.to_bytes().to_vec();
        Ok(Body { data })
    }

    /// Get body as UTF-8 string
    pub fn as_str(&self) -> Result<&str, String> {
        std::str::from_utf8(&self.data)
            .map_err(|e| format!("Invalid UTF-8: {}", e))
    }

    /// Get raw body bytes
    pub fn as_bytes(&self) -> &[u8] {
        &self.data
    }

    /// Parse body as JSON
    /// 
    /// # Example
    /// ```
    /// let body = Body::from_incoming(req.into_body()).await?;
    /// let user: User = body.json()?;
    /// ```
    pub fn json<T: DeserializeOwned>(&self) -> Result<T, String> {
        let text = self.as_str()?;
        serde_json::from_str(text)
            .map_err(|e| format!("Failed to parse JSON: {}", e))
    }

    /// Parse body as form data (application/x-www-form-urlencoded)
    pub fn form(&self) -> Result<HashMap<String, String>, String> {
        let text = self.as_str()?;
        Ok(Self::parse_form_data(text))
    }

    /// Check if body is empty
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    /// Get body size in bytes
    pub fn len(&self) -> usize {
        self.data.len()
    }

    // === Private Helpers ===

    fn parse_form_data(text: &str) -> HashMap<String, String> {
        text.split('&')
            .filter_map(Self::parse_form_pair)
            .collect()
    }

    fn parse_form_pair(pair: &str) -> Option<(String, String)> {
        pair.split_once('=').map(|(key, value)| {
            (
                urlencoding::decode(key).unwrap_or_default().to_string(),
                urlencoding::decode(value).unwrap_or_default().to_string(),
            )
        })
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
