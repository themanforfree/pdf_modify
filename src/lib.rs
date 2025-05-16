pub(crate) mod byte_range;
pub(crate) mod config;
pub(crate) mod manager;
pub(crate) mod parser;
pub(crate) mod signer;
pub(crate) mod utils;

pub use manager::PDFSignManager;
pub use manager::sign_info::{SignerInfo, SignerInfoBuilder};
pub use signer::{P12Signer, Sign};
