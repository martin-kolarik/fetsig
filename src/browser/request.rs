use std::time::Duration;

use js_sys::Uint8Array;
use log::warn;
use smol_str::{SmolStr, ToSmolStr};
use wasm_bindgen::JsValue;
use wasm_bindgen_futures::JsFuture;
use web_sys::{Headers, RequestInit};

use crate::{HEADER_WANTS_RESPONSE, MediaType};

use super::{
    common::{Abort, PendingFetch},
    file::File,
    js_error,
};

pub enum Method {
    Head,
    Get,
    Post,
    Put,
    Delete,
    Options,
}

impl Method {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Head => "Head",
            Self::Get => "Get",
            Self::Post => "Post",
            Self::Put => "Put",
            Self::Delete => "Delete",
            Self::Options => "Options",
        }
    }

    pub fn is_load(&self) -> bool {
        matches!(self, Self::Head | Self::Get | Self::Options)
    }
}

const HEADER_ACCEPT: &str = "Accept";
const HEADER_CONTENT_TYPE: &str = "Content-Type";

pub struct Request<'a> {
    logging: bool,
    method: Method,
    is_load: bool,
    url: &'a str,
    headers: Option<Vec<(&'static str, SmolStr)>>,
    media_type: Option<MediaType>,
    body: Option<Body>,
    wants_response: bool,
    timeout: Option<Duration>,
}

enum Body {
    Bytes(Vec<u8>),
    File(File),
}

impl<'a> Request<'a> {
    pub fn new(url: &'a str) -> Self {
        Self {
            logging: true,
            method: Method::Get,
            is_load: true,
            url,
            headers: None,
            media_type: None,
            body: None,
            wants_response: false,
            timeout: Some(Duration::from_secs(5)),
        }
    }

    #[must_use]
    pub fn with_logging(mut self, logging: bool) -> Self {
        self.logging = logging;
        self
    }

    #[must_use]
    pub fn with_method(mut self, method: Method) -> Self {
        self.method = method;
        self
    }

    #[must_use]
    pub fn with_header(mut self, name: &'static str, value: impl ToSmolStr) -> Self {
        let mut headers = self.headers.take().unwrap_or_default();
        headers.push((name, value.to_smolstr()));
        self.headers = Some(headers);
        self
    }

    #[must_use]
    pub fn with_headers(mut self, headers: Option<Vec<(&'static str, SmolStr)>>) -> Self {
        self.headers = headers;
        self
    }

    #[must_use]
    pub fn with_media_type(mut self, media_type: MediaType) -> Self {
        self.media_type = Some(media_type);
        self.with_header(HEADER_CONTENT_TYPE, media_type)
    }

    #[must_use]
    pub fn with_body(mut self, body: Vec<u8>) -> Self {
        self.body = Some(Body::Bytes(body));
        self
    }

    #[must_use]
    pub fn with_file(mut self, file: File) -> Self {
        self.body = Some(Body::File(file));
        self
    }

    #[must_use]
    pub fn with_is_load(mut self, is_load: bool) -> Self {
        self.is_load = is_load;
        self
    }

    #[must_use]
    pub fn with_timeout(mut self, timeout: Option<Duration>) -> Self {
        self.timeout = timeout;
        self
    }

    #[must_use]
    pub fn encoding(mut self, media_type: MediaType) -> Self {
        let media_type = match media_type {
            #[cfg(feature = "json")]
            MediaType::Json => MediaType::Json,
            #[cfg(feature = "postcard")]
            MediaType::Postcard => MediaType::Postcard,
            _ => {
                warn!(
                    "Unsupported media type '{media_type}' used, degrading to 'application/json'",
                );
                MediaType::Json
            }
        };
        self.wants_response = false;
        self.with_media_type(media_type)
            .with_header(HEADER_ACCEPT, media_type)
    }

    #[must_use]
    pub fn encoding_with_response(mut self, media_type: MediaType) -> Self {
        let media_type = match media_type {
            #[cfg(feature = "json")]
            MediaType::Json => MediaType::Json,
            #[cfg(feature = "postcard")]
            MediaType::Postcard => MediaType::Postcard,
            _ => {
                warn!(
                    "Unsupported media type '{media_type}' used, degrading to 'application/json'",
                );
                MediaType::Json
            }
        };
        self.wants_response = true;
        self.with_media_type(media_type)
            .with_header(HEADER_ACCEPT, media_type)
            .with_header(HEADER_WANTS_RESPONSE, "1")
    }

    #[cfg(feature = "json")]
    #[inline]
    #[must_use]
    pub fn json(self) -> Self {
        self.encoding(MediaType::Json)
    }

    #[cfg(feature = "json")]
    #[inline]
    #[must_use]
    pub fn json_with_response(self) -> Self {
        self.encoding_with_response(MediaType::Json)
    }

    #[cfg(feature = "postcard")]
    #[inline]
    #[must_use]
    pub fn postcard(self) -> Self {
        self.encoding(MediaType::Postcard)
    }

    #[cfg(feature = "postcard")]
    #[inline]
    #[must_use]
    pub fn postcard_with_response(self) -> Self {
        self.encoding_with_response(MediaType::Postcard)
    }

    #[must_use]
    pub fn create(self) -> Self {
        self.with_method(Method::Post)
    }

    #[must_use]
    pub fn retrieve(self) -> Self {
        self.with_method(Method::Get)
    }

    #[must_use]
    pub fn update(self) -> Self {
        self.with_method(Method::Put)
    }

    #[must_use]
    pub fn delete(self) -> Self {
        self.with_method(Method::Delete)
    }

    #[must_use]
    pub fn execute(self) -> Self {
        self.with_method(Method::Post)
    }

    pub fn logging(&self) -> bool {
        self.logging
    }

    pub fn method(&self) -> &Method {
        &self.method
    }

    pub fn is_load(&self) -> bool {
        self.is_load
    }

    pub fn url(&self) -> &str {
        self.url
    }

    pub fn media_type(&self) -> Option<MediaType> {
        self.media_type
    }

    pub fn headers(&self) -> Option<&[(&'static str, SmolStr)]> {
        self.headers.as_deref()
    }

    pub fn wants_response(&self) -> bool {
        self.wants_response
    }

    pub(crate) fn start(&self) -> Result<PendingFetch, SmolStr> {
        let request_init = RequestInit::new();
        request_init.set_method(match &self.method {
            Method::Head => "HEAD",
            Method::Get => "GET",
            Method::Post => "POST",
            Method::Put => "PUT",
            Method::Delete => "DELETE",
            Method::Options => "OPTIONS",
        });

        let headers: Headers = self.try_into()?;
        request_init.set_headers(&headers);

        if let Some(body) = &self.body {
            let value = match body {
                Body::Bytes(bytes) => {
                    let array: Uint8Array = bytes.as_slice().into();
                    JsValue::from(array)
                }
                Body::File(file) => JsValue::from(web_sys::File::from(file.clone())),
            };
            request_init.set_body(&value);
        }

        let abort = Abort::new()?;
        request_init.set_signal(Some(&abort.signal()));

        let promise = web_sys::window()
            .expect("window")
            .fetch_with_str_and_init(self.url(), &request_init);
        Ok(PendingFetch::new(
            self.url(),
            abort,
            self.timeout,
            JsFuture::from(promise),
        ))
    }
}

impl TryFrom<&Request<'_>> for Headers {
    type Error = SmolStr;

    fn try_from(request: &Request) -> Result<Self, Self::Error> {
        let output = Headers::new().map_err(js_error)?;
        if let Some(headers) = request.headers() {
            for (name, value) in headers {
                output.set(name, value).map_err(js_error)?;
            }
        }
        Ok(output)
    }
}
