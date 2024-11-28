use js_sys::Error;
use smol_str::{SmolStr, format_smolstr};
use wasm_bindgen::JsValue;

mod collection;
pub use collection::*;

mod collectionstate;
pub use collectionstate::*;

mod common;
pub use common::{FetchDeserializable, decode_content, none};

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
