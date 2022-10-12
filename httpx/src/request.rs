use std::time::Duration;
use url::Url;

use crate::{HttpRequest, HttpResponse, TransportError};

pub fn call(request: HttpRequest, timeout: Duration) -> Result<HttpResponse, TransportError> {
    let url: Url = request.url().unwrap();
    let (request, body): (ureq::Request, Option<String>) = request.into();
    let request = request.timeout(timeout);

    let response: Result<ureq::Response, ureq::Error> = match body {
        Some(body) => request.send_string(&body),
        None => request.call(),
    };

    conv(response, url)
}

fn conv(
    res: Result<ureq::Response, ureq::Error>,
    url: Url,
) -> Result<HttpResponse, TransportError> {
    let response: ureq::Response = match res {
        Ok(response) => response,
        Err(e) => match e {
            ureq::Error::Status(_, response) => response,
            ureq::Error::Transport(trans) => match trans.kind() {
                ureq::ErrorKind::Dns => return Err(TransportError::Connection(url)),
                ureq::ErrorKind::ConnectionFailed => return Err(TransportError::Connection(url)),

                ureq::ErrorKind::Io => return Err(TransportError::Connection(url)),
                _ => {
                    return Err(TransportError::Other(
                        trans.message().unwrap_or("Unknown transport error").to_string(),
                    ))
                }
            },
        },
    };

    let response: HttpResponse = response.into();
    Ok(response)
}
