use anyhow::Result;

use pdf_modify::{PDFSignManager, SignerInfo};

fn main() -> Result<()> {
    let pdf_path = "files/hello_world.pdf";
    let cert_path = "certs/mycert.p12";
    let output_path = "output/signed.pdf";
    let mut signer = PDFSignManager::load(pdf_path, cert_path)?;
    let signer_info = SignerInfo::builder()
        .name("John Doe")
        .reason("Signing Document")
        .contact_info("john.doe@example.com")
        .location("New York")
        .date("2025-05-15")
        .build();
    signer.sign(signer_info)?;
    signer.save(output_path)?;
    Ok(())
}
