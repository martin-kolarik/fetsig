use std::error::Error;
use wasm_bindgen::JsValue;

mod collection;
pub use collection::*;

mod collectionstate;
pub use collectionstate::*;

mod common;
pub use common::{decode_content, none};

mod entity;
pub use self::entity::*;

mod file;

mod mac;
pub use mac::*;

mod request;
pub use request::*;

mod transferstate;

mod upload;
pub use upload::*;

fn js_error(value: impl Into<JsValue>) -> String {
    Error::from(value.into()).to_string().into()
}

#[macro_export]
macro_rules! uformat {
    ($($arg:tt)*) => {{
        use ufmt;
        let mut text = String::new();
        ufmt::uwrite!(&mut text, $($arg)*).unwrap();
        text
    }}
}
