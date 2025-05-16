use std::path::Path;

use anyhow::Result;
use openssl::{
    cms::{CMSOptions, CmsContentInfo},
    pkcs12::{ParsedPkcs12_2, Pkcs12},
};

use super::Sign;

pub struct P12Signer {
    pkcs12: ParsedPkcs12_2,
}

impl P12Signer {
    pub fn load(path: impl AsRef<Path>) -> Result<Self> {
        Self::load_with_password(path, "")
    }
    pub fn load_with_password(path: impl AsRef<Path>, password: &str) -> Result<Self> {
        let der = std::fs::read(path.as_ref())?;
        let pkcs12 = Pkcs12::from_der(&der)?.parse2(password)?;
        Ok(P12Signer { pkcs12 })
    }
}

impl Sign for P12Signer {
    fn sign(&self, data: &[u8]) -> Result<Vec<u8>> {
        CmsContentInfo::sign(
            self.pkcs12.cert.as_deref(),
            self.pkcs12.pkey.as_deref(),
            self.pkcs12.ca.as_deref(),
            Some(data),
            CMSOptions::DETACHED | CMSOptions::BINARY,
        )?
        .to_der()
        .map_err(Into::into)
    }
}
