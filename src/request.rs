use hyper::body::Incoming;
use http_body_util::BodyExt;
use std::collections::HashMap;
use serde::de::DeserializeOwned;
use bytes::Bytes;

/// Custom Request wrapper that includes path parameters
pub struct Request {
    inner: hyper::Request<Incoming>,
    params: HashMap<String, String>,
}

impl Request {
    /// Create a new Request from a hyper Request
    pub fn new(inner: hyper::Request<Incoming>) -> Self {
        Self {
            inner,
            params: HashMap::new(),
        }
    }

    /// Create a new Request with path parameters
    pub fn with_params(inner: hyper::Request<Incoming>, params: HashMap<String, String>) -> Self {
        Self { inner, params }
    }

    /// Get a path parameter by name
    /// 
    /// # Example
    /// ```ignore
    /// // For route "/users/<id>"
    /// let id = req.param("id"); // Returns Some("123") for "/users/123"
    /// ```
    pub fn param(&self, name: &str) -> Option<&str> {
        self.params.get(name).map(|s| s.as_str())
    }

    /// Get a path parameter as a specific type
    /// 
    /// # Example
    /// ```ignore
    /// let id: Option<u64> = req.param_as("id");
    /// ```
    pub fn param_as<T: std::str::FromStr>(&self, name: &str) -> Option<T> {
        self.params.get(name).and_then(|s| s.parse().ok())
    }

    /// Get all path parameters
    pub fn params(&self) -> &HashMap<String, String> {
        &self.params
    }

    /// Get the request method
    pub fn method(&self) -> &hyper::Method {
        self.inner.method()
    }

    /// Get the request URI
    pub fn uri(&self) -> &hyper::Uri {
        self.inner.uri()
    }

    /// Get the request headers
    pub fn headers(&self) -> &hyper::HeaderMap {
        self.inner.headers()
    }

    /// Get a specific header value
    pub fn header(&self, name: &str) -> Option<&str> {
        self.inner.headers()
            .get(name)
            .and_then(|v| v.to_str().ok())
    }

    /// Consume the request and return the inner hyper Request
    pub fn into_inner(self) -> hyper::Request<Incoming> {
        self.inner
    }

    /// Get query parameters
    pub fn query(&self) -> Query {
        Query::from_uri(self.inner.uri())
    }

    /// Consume and read the body
    pub async fn into_body(self) -> Result<Body, String> {
        let (_, incoming) = self.inner.into_parts();
        Body::from_incoming(incoming).await
    }
}

/// Query parameter parser for URL queries - uses lazy evaluation to avoid parsing if unused
pub struct Query {
    raw_query: Option<String>,
    parsed: std::cell::RefCell<Option<HashMap<String, String>>>,
}

impl Query {
    /// Parse query parameters from a URI (lazy evaluation - only parses on first access)
    /// 
    /// # Example
    /// ```ignore
    /// use unipotato::Query;
    /// 
    /// let query = Query::from_uri(req.uri());
    /// ```
    pub fn from_uri(uri: &hyper::Uri) -> Self {
        let raw_query = uri.query().map(|s| s.to_string());
        Query {
            raw_query,
            parsed: std::cell::RefCell::new(None),
        }
    }

    /// Get a query parameter value (parses query string on first access, returns owned String)
    pub fn get(&self, key: &str) -> Option<String> {
        self.ensure_parsed();
        self.parsed.borrow()
            .as_ref()
            .and_then(|p| p.get(key).cloned())
    }

    /// Get a query parameter with a default value
    pub fn get_or(&self, key: &str, default: &str) -> String {
        self.ensure_parsed();
        self.parsed.borrow()
            .as_ref()
            .and_then(|p| p.get(key).cloned())
            .unwrap_or_else(|| default.to_string())
    }

    /// Check if a query parameter exists (parses on first access)
    pub fn has(&self, key: &str) -> bool {
        self.ensure_parsed();
        if let Some(params) = self.parsed.borrow().as_ref() {
            params.contains_key(key)
        } else {
            false
        }
    }

    /// Get all query parameters (parses query string on first access)
    pub fn all(&self) -> HashMap<String, String> {
        self.ensure_parsed();
        self.parsed.borrow()
            .as_ref()
            .cloned()
            .unwrap_or_default()
    }

    // === Private Helpers ===

    /// Ensure query string is parsed (only parses once, on first access)
    fn ensure_parsed(&self) {
        if self.parsed.borrow().is_none() {
            if let Some(raw_query) = &self.raw_query {
                let params = Self::parse_query_string(raw_query);
                *self.parsed.borrow_mut() = Some(params);
            }
        }
    }

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

/// HTTP request body parser - uses Bytes for zero-copy allocation
pub struct Body {
    data: Bytes,
}

impl Body {
    /// Read body from incoming request with optimized zero-copy buffering
    pub async fn from_incoming(incoming: Incoming) -> Result<Self, String> {
        let collected = incoming.collect().await
            .map_err(|e| format!("Failed to read body: {}", e))?;
        
        // Use to_bytes() directly instead of converting to Vec then back to Bytes
        // This avoids unnecessary allocations and provides reference-counted memory
        let data = collected.to_bytes();
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
    /// ```ignore
    /// use unipotato::Body;
    /// use serde::Deserialize;
    /// 
    /// #[derive(Deserialize)]
    /// struct User { name: String }
    /// 
    /// async fn handler(req: hyper::Request<hyper::body::Incoming>) -> Result<(), Box<dyn std::error::Error>> {
    ///     let body = Body::from_incoming(req.into_body()).await?;
    ///     let user: User = body.json()?;
    ///     Ok(())
    /// }
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

#[cfg(test)]
mod tests {
    use super::*;

    // ============ Query Parameter Parsing Tests ============

    #[test]
    fn query_parses_and_decodes_values() {
        let uri: hyper::Uri = "/users?id=42&name=John%20Doe&email=test%40example.com"
            .parse()
            .unwrap();
        let query = Query::from_uri(&uri);

        assert_eq!(query.get("id"), Some(&"42".to_string()));
        assert_eq!(query.get("name"), Some(&"John Doe".to_string()));
        assert_eq!(query.get("email"), Some(&"test@example.com".to_string()));
        assert!(query.has("id"));
        assert_eq!(query.get_or("missing", "fallback"), "fallback");
    }

    #[test]
    fn query_ignores_malformed_pairs_without_equals() {
        let uri: hyper::Uri = "/search?q=rust&malformed&lang=en".parse().unwrap();
        let query = Query::from_uri(&uri);

        assert_eq!(query.get("q"), Some(&"rust".to_string()));
        assert_eq!(query.get("lang"), Some(&"en".to_string()));
        assert!(!query.has("malformed"));
    }

    #[test]
    fn query_duplicate_keys_last_value_wins() {
        let uri: hyper::Uri = "/items?tag=rust&tag=web&tag=ml".parse().unwrap();
        let query = Query::from_uri(&uri);

        // HashMap collection semantics keep the most recent value for duplicate keys.
        assert_eq!(query.get("tag"), Some(&"ml".to_string()));
    }

    // ============ Body Parsing Tests ============

    #[test]
    fn body_json_and_form_parsing() {
        let json_body = Body {
            data: br#"{"count":3,"enabled":true}"#.to_vec(),
        };
        let parsed: serde_json::Value = json_body.json().unwrap();
        assert_eq!(parsed["count"], 3);
        assert_eq!(parsed["enabled"], true);

        let form_body = Body {
            data: b"name=John%20Doe&role=admin".to_vec(),
        };
        let form = form_body.form().unwrap();
        assert_eq!(form.get("name"), Some(&"John Doe".to_string()));
        assert_eq!(form.get("role"), Some(&"admin".to_string()));
    }

    #[test]
    fn body_form_ignores_malformed_pairs() {
        let form_body = Body {
            data: b"name=John&malformed&role=admin".to_vec(),
        };

        let form = form_body.form().unwrap();
        assert_eq!(form.get("name"), Some(&"John".to_string()));
        assert_eq!(form.get("role"), Some(&"admin".to_string()));
        assert!(!form.contains_key("malformed"));
    }

    #[test]
    fn body_empty_and_len_reflect_payload_size() {
        let empty = Body { data: Vec::new() };
        assert!(empty.is_empty());
        assert_eq!(empty.len(), 0);

        let non_empty = Body {
            data: b"abc".to_vec(),
        };
        assert!(!non_empty.is_empty());
        assert_eq!(non_empty.len(), 3);
    }

    // ============ Body Error Handling Tests ============

    #[test]
    fn body_reports_utf8_and_json_errors() {
        let invalid_utf8 = Body {
            data: vec![0xff, 0xfe, 0xfd],
        };
        assert!(invalid_utf8.as_str().is_err());

        let bad_json = Body {
            data: br#"{"count":}"#.to_vec(),
        };
        assert!(bad_json.json::<serde_json::Value>().is_err());
    }
}
