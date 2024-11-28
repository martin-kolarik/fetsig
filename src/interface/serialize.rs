#[cfg(feature = "json")]
pub use json::*;
#[cfg(feature = "json")]
mod json {
    use std::io::Write;

    use serde::{Serialize, de::DeserializeOwned};
    use smol_str::SmolStr;

    use crate::uformat_smolstr;

    pub trait JSONSerialize
    where
        Self: Serialize,
    {
        fn write_json<W: Write>(&self, writer: &mut W) -> Result<(), SmolStr> {
            serde_json::to_writer(writer, self)
                .map_err(|e| uformat_smolstr!("Serialization (json) failed: {}", e.to_string()))
        }

        fn to_json(&self) -> Vec<u8> {
            let mut buffer = Vec::with_capacity(8192);
            match self.write_json(&mut buffer) {
                Ok(_) => buffer,
                Err(_) => vec![],
            }
        }
    }

    pub trait JSONDeserialize
    where
        Self: DeserializeOwned,
    {
        fn try_from_json(json: &[u8]) -> Result<Self, SmolStr> {
            serde_json::from_slice::<Self>(json)
                .map_err(|e| uformat_smolstr!("Deserialization (json) failed: {}", e.to_string()))
        }
    }

    impl<E> JSONSerialize for E where E: Serialize {}
    impl<E> JSONDeserialize for E where E: DeserializeOwned {}
}

#[cfg(feature = "postcard")]
pub use postcard::*;
#[cfg(feature = "postcard")]
mod postcard {
    use std::io::Write;

    use postcard::{ser_flavors::Flavor, serialize_with_flavor};
    use serde::{Serialize, de::DeserializeOwned};
    use smol_str::SmolStr;

    use crate::uformat_smolstr;

    struct PostcardWriteStorage<'a, W> {
        writer: &'a mut W,
    }

    impl<W> Flavor for PostcardWriteStorage<'_, W>
    where
        W: Write,
    {
        type Output = ();

        fn try_push(&mut self, data: u8) -> postcard::Result<()> {
            self.try_extend(&[data])
        }

        fn finalize(self) -> postcard::Result<Self::Output> {
            Ok(())
        }

        fn try_extend(&mut self, data: &[u8]) -> postcard::Result<()> {
            match self.writer.write_all(data) {
                Ok(_) => Ok(()),
                Err(_) => Err(postcard::Error::SerializeBufferFull),
            }
        }
    }

    pub trait PostcardSerialize
    where
        Self: Serialize,
    {
        fn write_postcard<W: Write>(&self, writer: &mut W) -> Result<(), SmolStr> {
            let storage = PostcardWriteStorage { writer };
            serialize_with_flavor(self, storage)
                .map_err(|e| uformat_smolstr!("Serialization (postcard) failed: {}", e.to_string()))
        }

        fn to_postcard(&self) -> Vec<u8> {
            let mut buffer = Vec::with_capacity(4096);
            match self.write_postcard(&mut buffer) {
                Ok(_) => buffer,
                Err(_) => vec![],
            }
        }
    }

    pub trait PostcardDeserialize
    where
        Self: DeserializeOwned,
    {
        fn try_from_postcard(postcard: &[u8]) -> Result<Self, SmolStr> {
            postcard::from_bytes::<Self>(postcard).map_err(|e| {
                uformat_smolstr!("Deserialization (postcard) failed: {}", e.to_string())
            })
        }
    }

    impl<E> PostcardSerialize for E where E: Serialize {}
    impl<E> PostcardDeserialize for E where E: DeserializeOwned {}
}
