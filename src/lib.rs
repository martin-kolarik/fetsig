#[cfg(feature = "browser")]
mod browser;
#[cfg(feature = "browser")]
pub use browser::*;

mod interface;
pub use interface::*;

pub use futures_signals_ext::*;
use smol_str::SmolStrBuilder;

use ufmt::uWrite;

#[macro_export]
macro_rules! uformat_smolstr {
    ($($arg:tt)*) => {{
        use {ufmt, smol_str};
        let mut builder = $crate::Ufmtf(smol_str::SmolStrBuilder::new());
        ufmt::uwrite!(&mut builder, $($arg)*).unwrap();
        builder.0.finish()
    }}
}

pub struct Ufmtf(pub SmolStrBuilder);

impl uWrite for Ufmtf {
    type Error = ();

    fn write_str(&mut self, s: &str) -> Result<(), ()> {
        self.0.push_str(s);
        Ok(())
    }
}

#[cfg(all(feature = "browser", not(feature = "json"), not(feature = "postcard")))]
compile_error!(
    "No serialization feature present, select at least one of 'json' or 'postcard' features."
);
