use std::convert::Infallible;

use url::Url;

use crate::http::HttpRequest;

impl TryFrom<HttpRequest> for ureq::Request {
    type Error = url::ParseError;

    fn try_from(req: HttpRequest) -> Result<Self, Self::Error> {
        let url: Url = req.url()?;

        let mut request = ureq::request_url(&req.verb().to_string(), &url);

        for (key, value) in req.headers() {
            request.set(key.as_str(), value.as_str());
        }

        Ok(request)
    }
}

pub fn send(request: ureq::Request, body: Option<String>) -> Result<ureq::Response, ureq::Error> {
    match body {
        Some(body) => request.send_string(&body),
        None => request.call(),
    }
}
