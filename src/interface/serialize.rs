use std::{io::Write, result::Result};

use bytes::{Bytes, BytesMut};
use postcard::{ser_flavors::Flavor, serialize_with_flavor};
use serde::Serialize;

pub use serde::de::DeserializeOwned;

use crate::uformat;

pub trait DeserializeByMimeType: DeserializeOwned + JSONDeserialize + PostcardDeserialize {}
impl<T> DeserializeByMimeType for T where T: DeserializeOwned + JSONDeserialize + PostcardDeserialize
{}

#[cfg(feature = "json")]
pub trait JSONSerialize
where
    Self: Serialize,
{
    fn write_json<W: Write>(&self, writer: &mut W) -> Result<(), String> {
        serde_json::to_writer(writer, self)
            .map_err(|e| uformat!("Serialization (json) failed: {}", e.to_string()))
    }

    fn to_json(&self) -> Bytes {
        let mut buffer = BytesMut::with_capacity(8192).writer();
        match self.write_json(&mut buffer) {
            Ok(_) => buffer.into_inner().freeze(),
            Err(_) => Bytes::new(),
        }
    }
}

#[cfg(feature = "json")]
pub trait JSONDeserialize
where
    Self: DeserializeOwned,
{
    fn try_from_json(json: &[u8]) -> Result<Self, String> {
        serde_json::from_slice::<Self>(json)
            .map_err(|e| uformat!("Deserialization (json) failed: {}", e.to_string()))
    }
}

impl<E> JSONSerialize for E where E: Serialize {}
impl<E> JSONDeserialize for E where E: DeserializeOwned {}

#[cfg(feature = "postcard")]
struct PostcardWriteStorage<'a, W> {
    writer: &'a mut W,
}

#[cfg(feature = "postcard")]
impl<'a, W> Flavor for PostcardWriteStorage<'a, W>
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
        match self.writer.write(data) {
            Ok(_) => Ok(()),
            Err(_) => Err(postcard::Error::SerializeBufferFull),
        }
    }
}

#[cfg(feature = "postcard")]
pub trait PostcardSerialize
where
    Self: Serialize,
{
    fn write_postcard<W: Write>(&self, writer: &mut W) -> Result<()> {
        let storage = PostcardWriteStorage { writer };
        serialize_with_flavor(self, storage)
            .map_err(|e| uformat!("Serialization (postcard) failed: {}", e.to_string()))
    }

    fn to_postcard(&self) -> Bytes {
        let mut buffer = BytesMut::with_capacity(4096).writer();
        match self.write_postcard(&mut buffer) {
            Ok(_) => buffer.into_inner().freeze(),
            Err(_) => Bytes::new(),
        }
    }
}

#[cfg(feature = "postcard")]
pub trait PostcardDeserialize
where
    Self: DeserializeOwned,
{
    fn try_from_postcard(postcard: &[u8]) -> Result<Self> {
        postcard::from_bytes::<Self>(postcard)
            .map_err(|e| uformat!("Deserialization (postcard) failed: {}", e.to_string()))
    }
}

impl<E> PostcardSerialize for E where E: Serialize {}
impl<E> PostcardDeserialize for E where E: DeserializeOwned {}
