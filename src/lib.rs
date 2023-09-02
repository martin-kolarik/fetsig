#[cfg(feature = "browser")]
mod browser;
#[cfg(feature = "browser")]
pub use browser::*;

mod interface;
pub use interface::*;

pub use futures_signals_ext::*;

#[macro_export]
macro_rules! uformat {
    ($($arg:tt)*) => {{
        use ufmt;
        let mut text = String::new();
        ufmt::uwrite!(&mut text, $($arg)*).unwrap();
        text
    }}
}

#[cfg(all(feature = "browser", not(feature = "json"), not(feature = "postcard")))]
compile_error!(
    "No serialization feature present, select at least one of 'json' or 'postcard' features."
);
