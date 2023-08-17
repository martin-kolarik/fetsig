pub trait MacSign {
    fn sign(_message: &[u8]) -> Option<String> {
        None
    }
}

pub trait MacVerify {
    fn verify(_message: &[u8], _signature: Option<&str>) -> Result<bool, String> {
        Ok(true)
    }
}

#[derive(Debug)]
pub struct NoMac;

impl MacSign for NoMac {}
impl MacVerify for NoMac {}
