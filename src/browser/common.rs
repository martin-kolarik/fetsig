use std::time::Duration;

use base64::{engine::general_purpose, Engine};
use futures_time::future::FutureExt;
use gloo_timers::future::TimeoutFuture;
use js_sys::{JsString, Uint8Array};
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::JsFuture;
use web_sys::{AbortController, AbortSignal, Response, ResponseType};

use crate::{uformat, MacVerify, MediaType, StatusCode, HEADER_SIGNATURE};

#[cfg(feature = "json")]
use crate::JSONDeserialize;

#[cfg(feature = "postcard")]
use crate::PostcardDeserialize;

use super::js_error;
pub fn none(_: StatusCode) {}

#[cfg(all(feature = "json", feature = "postcard"))]
pub trait FetchDeserializable: JSONDeserialize + PostcardDeserialize {}
#[cfg(all(feature = "json", feature = "postcard"))]
impl<F> FetchDeserializable for F where F: JSONDeserialize + PostcardDeserialize {}

#[cfg(all(feature = "json", not(feature = "postcard")))]
pub trait FetchDeserializable: JSONDeserialize {}
#[cfg(all(feature = "json", not(feature = "postcard")))]
impl<F> FetchDeserializable for F where F: JSONDeserialize {}

#[cfg(all(not(feature = "json"), feature = "postcard"))]
pub trait FetchDeserializable: PostcardDeserialize {}
#[cfg(all(not(feature = "json"), feature = "postcard"))]
impl<F> FetchDeserializable for F where F: PostcardDeserialize {}

pub struct Abort {
    controller: AbortController,
}

impl Abort {
    pub fn new() -> Result<Self, String> {
        Ok(Self {
            controller: AbortController::new().map_err(js_error)?,
        })
    }

    pub fn signal(&self) -> AbortSignal {
        self.controller.signal()
    }

    pub fn abort(&self) {
        self.controller.abort()
    }
}

pub(crate) struct PendingFetch {
    url: String,
    #[allow(dead_code)]
    abort: Abort,
    timeout: Option<Duration>,
    request_future: JsFuture,
}

impl PendingFetch {
    pub fn new(
        url: impl ToString,
        abort: Abort,
        timeout: Option<Duration>,
        request_future: JsFuture,
    ) -> Self {
        Self {
            url: url.to_string(),
            abort,
            timeout,
            request_future,
        }
    }

    pub async fn wait_completion(self) -> DecodedResponse<Response> {
        let timeout = TimeoutFuture::new(
            self.timeout
                .unwrap_or_else(|| Duration::from_secs(900))
                .as_millis() as u32,
        );
        match self.request_future.timeout(timeout).await {
            Ok(Ok(response)) => {
                let response = response.unchecked_into::<Response>();
                if !response.ok() && matches!(response.type_(), ResponseType::Error) {
                    DecodedResponse::new(StatusCode::FetchFailed).with_hint("Fetch network error")
                } else {
                    DecodedResponse::new(response.status()).with_response(response)
                }
            }
            Ok(Err(error)) => DecodedResponse::new(StatusCode::FetchFailed)
                .with_hint(uformat!("Fetch start failed ({})", js_error(error))),
            Err(_) => {
                self.abort.abort();
                DecodedResponse::new(StatusCode::FetchTimeout).with_hint(self.url)
            }
        }
    }
}

pub(crate) struct DecodedResponse<R> {
    status: StatusCode,
    hint: Option<String>,
    response: Option<R>,
}

impl<R> DecodedResponse<R> {
    pub fn new(status: impl Into<StatusCode>) -> Self {
        Self {
            status: status.into(),
            hint: None,
            response: None,
        }
    }

    pub fn with_response(mut self, response: R) -> Self {
        self.response = Some(response);
        self
    }

    pub fn with_hint(mut self, hint: impl ToString) -> Self {
        self.hint = Some(hint.to_string());
        self
    }

    pub fn status(&self) -> StatusCode {
        self.status
    }

    pub fn take_response(&mut self) -> Option<R> {
        self.response.take()
    }

    pub fn hint(&self) -> Option<&str> {
        self.hint.as_deref()
    }

    fn cast_failure<U>(self) -> DecodedResponse<U> {
        DecodedResponse {
            status: self.status,
            hint: self.hint,
            response: None,
        }
    }
}

pub(crate) async fn execute_fetch<R, MV>(fetch: PendingFetch) -> DecodedResponse<R>
where
    R: FetchDeserializable,
    MV: MacVerify,
{
    let mut fetched = fetch.wait_completion().await;
    let Some(response) = fetched.take_response() else {
        return fetched.cast_failure();
    };

    let status = fetched.status();
    match status {
        StatusCode::Ok
        | StatusCode::Created
        | StatusCode::NoContent
        | StatusCode::BadRequest
        | StatusCode::Forbidden
        | StatusCode::InternalServerError
        | StatusCode::NotFound
        | StatusCode::PayloadTooBig
        | StatusCode::RateLimited
        | StatusCode::Unauthorized => match decode_response::<R, MV>(status, response).await {
            Ok(result) => result,
            Err(result) => result,
        },
        _ => fetched.cast_failure(),
    }
}

async fn decode_response<R, MV>(
    status: StatusCode,
    response: Response,
) -> Result<DecodedResponse<R>, DecodedResponse<R>>
where
    R: FetchDeserializable,
    MV: MacVerify,
{
    let headers = response.headers();
    let content_type = headers.get("Content-Type").map_err(|error| {
        DecodedResponse::new(StatusCode::FetchFailed).with_hint(uformat!(
            "Cannot decode Content-Type header: {}.",
            js_error(error)
        ))
    })?;
    let media_type = match content_type {
        Some(content_type) => MediaType::from(content_type.as_str()),
        None => MediaType::Plain,
    };

    let signature = headers.get(HEADER_SIGNATURE).map_err(|error| {
        DecodedResponse::new(StatusCode::FetchFailed).with_hint(uformat!(
            "Cannot decode {} header: {}.",
            HEADER_SIGNATURE,
            js_error(error)
        ))
    })?;

    let array_promise = response
        .array_buffer()
        .map_err(|_| DecodedResponse::new(StatusCode::DecodeFailed).with_hint("Decode 1"))?;
    let content_array_buffer = JsFuture::from(array_promise)
        .await
        .map_err(|_| DecodedResponse::new(StatusCode::DecodeFailed).with_hint("Decode 2"))?;

    match decode_content::<_, MV>(
        media_type,
        false,
        content_array_buffer,
        signature.as_deref(),
    )
    .await
    {
        Ok(None) => Ok(DecodedResponse::new(status)),
        Ok(Some(response)) => Ok(DecodedResponse::new(status).with_response(response)),
        Err((status, hint)) => Err(DecodedResponse::new(status).with_hint(hint)),
    }
}

pub async fn decode_content<R, MV>(
    media_type: MediaType,
    decode_base64: bool,
    content: JsValue,
    signature: Option<&str>,
) -> Result<Option<R>, (StatusCode, String)>
where
    R: FetchDeserializable,
    MV: MacVerify,
{
    match media_type {
        #[cfg(feature = "json")]
        MediaType::Json => (),
        #[cfg(feature = "postcard")]
        MediaType::Postcard => (),
        _ => Err((StatusCode::UnsupportedMediaType, String::default()))?,
    }

    let data = if content.is_string() {
        if let Some(string) = content.dyn_ref::<JsString>().and_then(|s| s.as_string()) {
            if string.len() == 0 {
                return Ok(None);
            } else {
                string.as_bytes().to_vec()
            }
        } else {
            return Ok(None);
        }
    } else {
        // otherwise content is an array buffer
        let array = Uint8Array::new(&content);
        if array.length() == 0 {
            return Ok(None);
        }
        array.to_vec()
    };

    let data = if decode_base64 {
        general_purpose::STANDARD_NO_PAD
            .decode(data)
            .map_err(|error| (StatusCode::DecodeFailed, error.to_string()))?
    } else {
        data
    };

    match MV::verify(&data, signature) {
        Ok(true) => {}
        Ok(false) => Err((
            StatusCode::DecodeFailed,
            "Response signature is invalid.".into(),
        ))?,
        Err(error) => Err((
            StatusCode::DecodeFailed,
            uformat!("Response signature verification failed: {}.", error),
        ))?,
    }

    match media_type {
        #[cfg(feature = "json")]
        MediaType::Json => R::try_from_json(&data).map_err(|e| e.to_string()),
        #[cfg(feature = "postcard")]
        MediaType::Postcard => R::try_from_postcard(&data).map_err(|e| e.to_string()),
        _ => {
            return Err((
                StatusCode::UnsupportedMediaType,
                "Decode/deserialize error, unexpected data flow for unsupported media type.".into(),
            ));
        }
    }
    .map_err(|error| {
        (
            StatusCode::DecodeFailed,
            uformat!("Deserialization failed: {}", error),
        )
    })
    .map(|response| Some(response))
}
