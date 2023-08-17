mod mediatype;
pub use mediatype::*;

mod serialize;
pub use serialize::*;

mod statuscode;
pub use statuscode::*;

pub const HEADER_SIGNATURE: &str = "Content-Signature";
pub const HEADER_WANTS_RESPONSE: &str = "Wants-Response";
