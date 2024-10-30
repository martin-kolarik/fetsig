use js_sys::Error;
use smol_str::{format_smolstr, SmolStr};
use wasm_bindgen::JsValue;

mod collection;
pub use collection::*;

mod collectionstate;
pub use collectionstate::*;

mod common;
pub use common::{decode_content, none, FetchDeserializable};

mod entity;
pub use self::entity::*;

mod file;
pub use file::*;

mod mac;
pub use mac::*;

mod request;
pub use request::*;

mod transferstate;

mod upload;
pub use upload::*;

fn js_error(value: impl Into<JsValue>) -> SmolStr {
    format_smolstr!("{}", Error::from(value.into()).to_string())
}
