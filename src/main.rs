use std::{
    fs::{File, create_dir_all},
    io::Read,
    path::{Path, PathBuf},
    process::Command,
};

use anyhow::Result;

use chrono::{DateTime, Utc};
use clap::{Args, Parser, Subcommand, arg, command};
use lopdf::{Document, Object};
use pdf_modify::{ImageRect, P12Signer, PDFSignManager, SignerInfo};

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Sign {
            input,
            output,
            cert,
            signer_info,
            image,
            pos,
            size,
            cross_page_image,
            cross_page_size,
        } => {
            let mut manager = PDFSignManager::load(input)?;

            if let Some(img) = cross_page_image {
                manager.add_cross_page_seal(img, cross_page_size)?;
            };

            if let Some(cert) = cert {
                let signer = P12Signer::load(cert)?;
                let img = image.map(|p| ImageRect::new(p, pos, size));
                manager.sign(signer_info.into(), img, &signer)?;
            }
            if let Some(dir) = output.parent() {
                create_dir_all(dir)?;
            };
            manager.save(output)?;
        }
        Commands::Verify { input } => verify(input)?,
    };
    Ok(())
}

#[derive(Debug, Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// Sign a PDF document
    Sign {
        /// Path to the input PDF file
        #[arg(short, long)]
        input: PathBuf,
        /// Path to the output signed PDF file
        #[arg(short, long, default_value = "output/signed.pdf")]
        output: PathBuf,
        /// Path to the certificate file
        #[arg(short, long)]
        cert: Option<PathBuf>,
        /// Path to the seal image file
        #[arg(long)]
        image: Option<PathBuf>,
        /// Path to the cross page seal image file
        #[arg(long)]
        cross_page_image: Option<PathBuf>,
        /// Position of the signature image in the PDF
        #[arg(long, value_parser = parse_number_pair,
            default_value = "100,100")]
        pos: (i64, i64),
        /// Target size of the signature image in the PDF
        #[arg(long, value_parser = parse_number_pair,
            default_value = "100,100")]
        size: (i64, i64),
        /// Target size of the cross page signature image in the PDF
        /// This is the raw size of the image, It will be cut to every page
        #[arg(long, value_parser = parse_number_pair,
            default_value = "100,100")]
        cross_page_size: (i64, i64),
        /// Information about the signer
        #[command(flatten)]
        signer_info: Box<SignerInfoArgs>,
    },
    /// Verify a signed PDF document
    Verify {
        /// Path to the signed PDF file
        #[arg(short, long)]
        input: PathBuf,
    },
}

#[derive(Debug, Args)]
struct SignerInfoArgs {
    /// Name of the signer
    #[arg(long)]
    name: Option<String>,
    /// Reason for signing
    #[arg(long)]
    reason: Option<String>,
    /// Contact information of the signer
    #[arg(long)]
    contact_info: Option<String>,
    /// Location of the signer
    #[arg(long)]
    location: Option<String>,
    /// Date of signing
    #[arg(long, default_value_t = Utc::now())]
    date: DateTime<Utc>,
}

impl From<Box<SignerInfoArgs>> for SignerInfo {
    fn from(value: Box<SignerInfoArgs>) -> Self {
        SignerInfo {
            name: value.name,
            reason: value.reason,
            contact_info: value.contact_info,
            location: value.location,
            date: Some(value.date),
        }
    }
}

fn parse_number_pair(s: &str) -> Result<(i64, i64), String> {
    let parts: Vec<&str> = s.split(',').collect();
    if parts.len() != 2 {
        return Err(format!("Invalid format: {}", s));
    }
    let x = parts[0]
        .parse()
        .map_err(|_| format!("Invalid number: {}", parts[0]))?;
    let y = parts[1]
        .parse()
        .map_err(|_| format!("Invalid number: {}", parts[1]))?;
    Ok((x, y))
}

fn verify(path: impl AsRef<Path>) -> Result<()> {
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
