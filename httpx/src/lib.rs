pub mod request;

extern crate http;
extern crate serde;
extern crate url;

use std::{str::FromStr, time::Duration};

use http::{header::HeaderName, HeaderMap, HeaderValue, Method};
use serde::Deserialize;
use url::Url;

const USER_AGENT_KEY: &str = "user-agent";
const USER_AGENT: &str = "fire/0.1.0";
const CONTENT_LENGTH_KEY: &str = "content-length";
const HOST_KEY: &str = "host";

#[derive(Debug, Deserialize)]
pub struct HttpRequest {
    #[serde(alias = "verb")]
    #[serde(with = "http_serde::method")]
    method: Method,
    url: String,
    body: Option<String>,
    #[serde(default)]
    #[serde(with = "http_serde::header_map")]
    headers: HeaderMap,
}

impl HttpRequest {
    pub fn method(&self) -> Method {
        self.method.clone()
    }

    pub fn url(&self) -> Result<Url, url::ParseError> {
        if self.url.starts_with("http://") || self.url.starts_with("https://") {
            Url::parse(&self.url)
        } else {
            Url::parse(&format!("https://{}", &self.url))
        }
    }

    pub fn headers(&self) -> HeaderMap {
        self.headers.clone()
    }

    pub fn header(&self, key: &str) -> Option<&str> {
        let key = HeaderName::from_str(key).ok()?;
        self.headers.get(key).and_then(|v| v.to_str().ok())
    }

    /// Set the _default_ values for headers:
    /// - `user-agent`
    /// - `content-length` (if request has a body)
    /// - `host` (if request URL contains a hostname)
    ///
    /// These default values will only be used if no explicit values are set in the request.
    pub fn set_default_headers(&mut self) -> Result<(), InvalidHeader> {
        let mut default: Vec<Header> = Vec::with_capacity(3);

        if let Some(host) = self.url().unwrap().host_str() {
            default.push(header(HOST_KEY, host)?);
        }

        if self.has_body() {
            let content_length = self.body_size().to_string();
            default.push(header(CONTENT_LENGTH_KEY, &content_length)?);
        }

        default.push(header(USER_AGENT_KEY, USER_AGENT)?);

        default.into_iter().for_each(|(key, value)| {
            self.headers.entry(key).or_insert(value);
        });

        Ok(())
    }

    pub fn has_body(&self) -> bool {
        self.body_size() != 0
    }

    pub fn body(&self) -> &Option<String> {
        &self.body
    }

    pub fn body_size(&self) -> usize {
        match self.method {
            Method::PUT | Method::POST | Method::DELETE | Method::PATCH => match &self.body {
                Some(b) => b.len(),
                None => 0,
            },
            _ => 0,
        }
    }
}

pub type Header = (HeaderName, HeaderValue);

fn header(key: &str, value: &str) -> Result<Header, InvalidHeader> {
    let key = HeaderName::from_str(key).map_err(|_| InvalidHeader::Key(key.to_string()))?;
    let value =
        HeaderValue::from_str(value).map_err(|_| InvalidHeader::Value(value.to_string()))?;

    Ok((key, value))
}

#[derive(Debug)]
pub enum InvalidHeader {
    Key(String),
    Value(String),
}

impl FromStr for HttpRequest {
    type Err = serde_yaml::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        serde_yaml::from_str(s)
    }
}

impl From<HttpRequest> for (ureq::Request, Option<String>) {
    fn from(req: HttpRequest) -> Self {
        let url = req.url().unwrap();
        let request: ureq::Request = req
            .headers
            .iter()
            .fold(ureq::request(req.method.as_ref(), url.as_str()), |r, (key, value)| {
                r.set(key.as_str(), value.to_str().unwrap())
            });

        (request, req.body().clone())
    }
}

pub struct HttpResponse {
    version: String,
    status: u16,
    headers: HeaderMap,
    body: String,
}

impl HttpResponse {
    pub fn version(&self) -> &str {
        &self.version
    }

    pub fn status(&self) -> u16 {
        self.status
    }

    pub fn headers(&self) -> &HeaderMap {
        &self.headers
    }

    pub fn header(&self, key: &str) -> Option<&str> {
        let key = HeaderName::from_str(key).ok()?;
        self.headers.get(key).and_then(|v| v.to_str().ok())
    }

    pub fn body(&self) -> &str {
        &self.body
    }

    pub fn body_len(&self) -> usize {
        self.body.len()
    }
}

impl From<ureq::Response> for HttpResponse {
    fn from(resp: ureq::Response) -> Self {
        let version = resp.http_version().to_string();
        let resp_headers: Vec<String> = resp.headers_names();
        let headers: HeaderMap = resp_headers
            .into_iter()
            .map(|key| (key.clone(), resp.header(&key)))
            .filter_map(|(key, opt)| opt.map(|v| (key, v)))
            .filter_map(|(key, value)| header(&key, value).ok())
            .collect();

        // TODO: Log or notify somehow if resp_headers and header size is not the same.
        // If that is the case, it means that some of the headers could not be parsed.

        HttpResponse {
            version,
            status: resp.status(),
            headers,
            body: resp.into_string().unwrap_or_default(),
        }
    }
}

#[derive(Debug)]
pub enum TransportError {
    Timeout(Url, Duration),
    Connection(Url),
    UnknownHost(Url),
    Other(String),
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use http::Method;
    use url::Url;

    use super::HttpRequest;

    #[test]
    fn test_parse_request_from_str() {
        let input = r###"
            # This is a comment
            method: POST
            url: api.github.com/markdown
            headers:
              accept: application/vnd.github+json
              content-type: application/json

            body: >
              {
                  "text": "**Hello** _world_!",
                  "mode": "markdown"
              }
        "###;

        let mut request = HttpRequest::from_str(input).unwrap();
        request.set_default_headers().unwrap();

        assert_eq!(Method::POST, request.method());

        let expected_url = Url::parse("https://api.github.com/markdown").unwrap();
        let actual_url = request.url().unwrap();

        assert_eq!(expected_url, actual_url);

        let content_type = request.header("content-type").unwrap();
        let host = request.header("host").unwrap();
        let accept = request.header("accept").unwrap();
        let user_agent = request.header("user-agent").unwrap();
        let content_size: usize = request.header("content-length").unwrap().parse().unwrap();

        assert_eq!("application/json", content_type);
        assert_eq!("api.github.com", host);
        assert_eq!("application/vnd.github+json", accept);
        assert_eq!("fire/", &user_agent[..5]);
        assert!(content_size > 0);

        assert!(request.body().is_some())
    }
}
