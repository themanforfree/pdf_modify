use anyhow::Result;

use pdf_modify::manager::{PDFSignManager, SignerInfo};

fn main() -> Result<()> {
    let pdf_path = "files/hello_world.pdf";
    let cert_path = "files/user.p12";
    let output_path = "output/signed.pdf";
    let mut signer = PDFSignManager::load(pdf_path, cert_path)?;
    let signer_info = SignerInfo::new(
        "John Doe",
        "Signing Document",
        "john.doe@example.com",
        "New York",
        "2025-05-15",
    );
    signer.sign(signer_info)?;
    signer.save(output_path)?;
    Ok(())
}
