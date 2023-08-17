use serde::{Deserialize, Deserializer, Serialize};

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum MediaType {
    #[default]
    ByteStream,
    Cbor,
    Css,
    Form,
    FormMultipart,
    Html,
    Ico,
    Javascript,
    Jpeg,
    Json,
    Plain,
    Png,
    Postcard,
    Sse,
    Svg,
    Wasm,
    Xml,
}

const BYTE_STREAM: &str = "application/octet-stream";
const CBOR: &str = "application/cbor";
const CSS: &str = "text/css";
const FORM: &str = "application/x-www-form-urlencoded";
const MULTIPART_FORM: &str = "multipart/form-data";
const HTML: &str = "text/html";
const ICO: &str = "image/x-icon";
const JAVASCRIPT: &str = "application/javascript";
const JPEG: &str = "image/jpeg";
const JSON: &str = "application/json";
const PLAIN: &str = "text/plain";
const PNG: &str = "image/png";
const POSTCARD: &str = "application/x-postcard";
const SSE: &str = "text/event-stream";
const SVG: &str = "image/svg+xml";
const WASM: &str = "application/wasm";
const XML: &str = "application/xml";

impl From<&str> for MediaType {
    fn from(mime: &str) -> Self {
        match mime {
            BYTE_STREAM => Self::ByteStream,
            CBOR => Self::Cbor,
            CSS => Self::Css,
            FORM => Self::Form,
            MULTIPART_FORM => Self::FormMultipart,
            HTML => Self::Html,
            ICO => Self::Ico,
            JAVASCRIPT => Self::Javascript,
            JPEG => Self::Jpeg,
            JSON => Self::Json,
            PNG => Self::Png,
            POSTCARD => Self::Postcard,
            SSE => Self::Sse,
            SVG => Self::Svg,
            WASM => Self::Wasm,
            XML => Self::Xml,
            _ => Self::default(),
        }
    }
}

impl From<String> for MediaType {
    fn from(mime: String) -> Self {
        Self::from(mime.as_str())
    }
}

impl AsRef<str> for MediaType {
    fn as_ref(&self) -> &str {
        match self {
            MediaType::ByteStream => BYTE_STREAM,
            MediaType::Cbor => CBOR,
            MediaType::Css => CSS,
            MediaType::Form => FORM,
            MediaType::FormMultipart => MULTIPART_FORM,
            MediaType::Html => HTML,
            MediaType::Ico => ICO,
            MediaType::Javascript => JAVASCRIPT,
            MediaType::Jpeg => JPEG,
            MediaType::Json => JSON,
            MediaType::Plain => PLAIN,
            MediaType::Png => PNG,
            MediaType::Postcard => POSTCARD,
            MediaType::Sse => SSE,
            MediaType::Svg => SVG,
            MediaType::Wasm => WASM,
            MediaType::Xml => XML,
        }
    }
}

impl Serialize for MediaType {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.into())
    }
}

impl<'de> Deserialize<'de> for MediaType {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let str = <&'de str as Deserialize>::deserialize(deserializer)?;
        Ok(str.into())
    }
}
