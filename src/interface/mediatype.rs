use core::fmt::Display;

use serde::{Deserialize, Deserializer, Serialize};
use smol_str::SmolStr;
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
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
    Pdf,
    Plain,
    Png,
    Postcard,
    Pwg,
    Sse,
    Svg,
    Urf,
    Wasm,
    Xml,
    Xlsx,
    Zip,
    Zip7,
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
const PDF: &str = "application/pdf";
const PLAIN: &str = "text/plain";
const PNG: &str = "image/png";
const POSTCARD: &str = "application/x-postcard";
const PWG: &str = "image/pwg-raster";
const SSE: &str = "text/event-stream";
const SVG: &str = "image/svg+xml";
const URF: &str = "image/urf";
const WASM: &str = "application/wasm";
const XLSX: &str = "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet";
const XML: &str = "application/xml";
const ZIP: &str = "application/zip";
const ZIP_WIN: &str = "application/x-zip-compressed";
const ZIP_7: &str = "application/x-7z-compressed";

impl MediaType {
    pub fn as_str(&self) -> &str {
        self.as_ref()
    }
}

impl Display for MediaType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_ref())
    }
}

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
            PDF => Self::Pdf,
            PNG => Self::Png,
            POSTCARD => Self::Postcard,
            PWG => Self::Pwg,
            SSE => Self::Sse,
            SVG => Self::Svg,
            URF => Self::Urf,
            WASM => Self::Wasm,
            XML => Self::Xml,
            XLSX => Self::Xlsx,
            ZIP => Self::Zip,
            ZIP_WIN => Self::Zip,
            ZIP_7 => Self::Zip7,
            _ => Self::default(),
        }
    }
}

impl From<SmolStr> for MediaType {
    fn from(mime: SmolStr) -> Self {
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
            MediaType::Pdf => PDF,
            MediaType::Plain => PLAIN,
            MediaType::Png => PNG,
            MediaType::Postcard => POSTCARD,
            MediaType::Pwg => PWG,
            MediaType::Sse => SSE,
            MediaType::Svg => SVG,
            MediaType::Urf => URF,
            MediaType::Wasm => WASM,
            MediaType::Xml => XML,
            MediaType::Xlsx => XLSX,
            MediaType::Zip => ZIP,
            MediaType::Zip7 => ZIP_7,
        }
    }
}

impl Serialize for MediaType {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.as_ref())
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
