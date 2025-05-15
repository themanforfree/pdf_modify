//! RawPdf
//! Parse the raw PDF data and replace the placeholder with the signed data.
use std::io::Write;

use crate::{byte_range::ByteRange, config::SIG_CONTENTS_PLACEHOLDER_LEN, signer::Sign};
use anyhow::Result;

pub struct RawPdf {
    data: Vec<u8>,
    byte_range: ByteRange,
}

impl RawPdf {
    pub fn empty() -> Self {
        RawPdf {
            data: Vec::new(),
            byte_range: ByteRange::default(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    pub fn load_data(&mut self, data: Vec<u8>) -> Result<()> {
        self.data = data;
        self.byte_range = ByteRange::from_bytes(&self.data)?;
        let bytes = self.byte_range.get_bytes();
        self.data[self.byte_range.byte_range_start..self.byte_range.byte_range_end]
            .copy_from_slice(&bytes);
        Ok(())
    }

    fn get_data_to_sign(&self) -> Vec<u8> {
        let mut data_to_sign = Vec::new();
        data_to_sign.extend_from_slice(
            &self.data
                [self.byte_range.value[0]..self.byte_range.value[0] + self.byte_range.value[1]],
        );
        data_to_sign.extend_from_slice(
            &self.data
                [self.byte_range.value[2]..self.byte_range.value[2] + self.byte_range.value[3]],
        );
        data_to_sign
    }

    pub fn sign(&mut self, signer: &dyn Sign) -> Result<()> {
        let data_to_sign = self.get_data_to_sign();
        let signed_data = signer.sign(&data_to_sign)?;
        if signed_data.len() > SIG_CONTENTS_PLACEHOLDER_LEN {
            return Err(anyhow::anyhow!("Signed data is too long"));
        }
        let contents_hex = hex::encode(&signed_data);
        let contents_start = self.byte_range.value[1] + 1;
        self.data[contents_start..contents_start + contents_hex.len()]
            .copy_from_slice(contents_hex.as_bytes());
        Ok(())
    }

    #[inline]
    pub fn save_to<W: Write>(&mut self, target: &mut W) -> Result<()> {
        target.write_all(&self.data)?;
        Ok(())
    }

    #[inline]
    pub fn save(&mut self, path: &str) -> Result<()> {
        let mut file = std::fs::File::create(path)?;
        self.save_to(&mut file)?;
        Ok(())
    }
}
