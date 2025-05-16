use anyhow::Result;

pub use self::p12::P12Signer;

mod p12;

pub trait Sign {
    fn sign(&self, data: &[u8]) -> Result<Vec<u8>>;
}
