#[cfg(feature = "browser")]
mod browser;
#[cfg(feature = "browser")]
pub use browser::*;

mod interface;
pub use interface::*;

#[macro_export]
macro_rules! uformat {
    ($($arg:tt)*) => {{
        use ufmt;
        let mut text = String::new();
        ufmt::uwrite!(&mut text, $($arg)*).unwrap();
        text
    }}
}

#[cfg(all(feature = "log", feature = "tracing"))]
compile_error!("Both 'log' and 'tracing' features selected, choose single one.");

#[cfg(all(not(feature = "log"), not(feature = "tracing")))]
compile_error!("No log features present, select either 'log' or 'tracing' feature.");

#[cfg(all(feature = "browser", not(feature = "json"), not(feature = "postcard")))]
compile_error!("No serialization feature present, select at least one of 'json' or 'postcard' features.");
