use anyhow::Result;
use lopdf::{Document, Object};
use std::{fs::File, io::Read, process::Command};

fn main() -> Result<()> {
    let path = std::env::args().nth(1).unwrap();
    let doc = Document::load(&path)?;
    let acro_dict = match doc.catalog()?.get(b"AcroForm")? {
        Object::Reference(id) => doc.get_dictionary(*id)?,
        Object::Dictionary(d) => d,
        _ => panic!("Invalid AcroForm id"),
    };
    let field_id = acro_dict.get(b"Fields")?.as_array()?[0].as_reference()?;
    let v_id = doc.get_dictionary(field_id)?.get(b"V")?.as_reference()?;
    let v_dict = doc.get_dictionary(v_id)?;
    let sign_contents = v_dict.get(b"Contents")?.as_str()?;
    let byte_range = v_dict
        .get(b"ByteRange")?
        .as_array()?
        .iter()
        .map(|obj| obj.as_i64().map(|x| x as usize))
        .collect::<Result<Vec<_>, _>>()?;

    let mut file_bytes = vec![];
    File::open(path)?.read_to_end(&mut file_bytes)?;

    let mut signed_bytes = Vec::new();
    signed_bytes.extend_from_slice(&file_bytes[byte_range[0]..byte_range[0] + byte_range[1]]);
    signed_bytes.extend_from_slice(&file_bytes[byte_range[2]..byte_range[2] + byte_range[3]]);

    std::fs::write("signature.der", sign_contents)?;
    std::fs::write("signed_content.bin", &signed_bytes)?;

    let output = Command::new("openssl")
        .args([
            "smime",
            "-verify",
            "-in",
            "signature.der",
            "-inform",
            "DER",
            "-content",
            "signed_content.bin",
            "-noverify",
        ])
        .output()?;

    if output.status.success() {
        println!("✅ 签名验证成功！");
    } else {
        println!("❌ 签名验证失败！");
    }
    std::fs::remove_file("signature.der")?;
    std::fs::remove_file("signed_content.bin")?;
    println!("{}", String::from_utf8_lossy(&output.stderr));
    Ok(())
}
