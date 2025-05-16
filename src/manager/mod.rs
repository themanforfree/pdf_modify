use std::io::Write;

use anyhow::Result;
use lopdf::{
    IncrementalDocument, Object, ObjectId, Stream, StringFormat,
    content::{Content, Operation},
    dictionary,
};

use self::image::ImageHelper;
pub use self::sign_info::SignerInfo;
use crate::{
    config::{SEAL_POS, SEAL_SIZE, SIG_CONTENTS_PLACEHOLDER_LEN},
    parser::RawPdf,
    signer::{P12Signer, Sign},
    utils::{AcroForm, Page, PageMut},
};

pub(crate) mod image;
pub(crate) mod sign_info;

pub struct PDFSignManager {
    doc: IncrementalDocument,
    raw_pdf: RawPdf,
    signer: Box<dyn Sign>,
}

impl PDFSignManager {
    pub fn load(pdf_path: &str, cert_path: &str) -> Result<Self> {
        // Load the PDF document and certificate
        let mut doc = IncrementalDocument::load(pdf_path)?;
        doc.new_document.version = "1.7".into();
        let signer = Box::new(P12Signer::load(cert_path)?);
        let raw_pdf = RawPdf::empty();
        Ok(PDFSignManager {
            doc,
            raw_pdf,
            signer,
        })
    }

    fn add_sig_obj(&mut self, signer_info: SignerInfo) -> ObjectId {
        let byte_range_placeholder = vec![
            Object::Integer(0),
            Object::Name("**********".into()),
            Object::Name("**********".into()),
            Object::Name("**********".into()),
        ];
        let sig_placeholder = vec![b'0'; SIG_CONTENTS_PLACEHOLDER_LEN];

        let mut sig_dict = dictionary! {
            "Type" => "Sig",
            "Filter" => "Adobe.PPKLite",
            "SubFilter" => "adbe.pkcs7.detached",
            "ByteRange" =>  byte_range_placeholder,
            "Contents" => Object::String(sig_placeholder, StringFormat::Hexadecimal),
            "Prop_Build" => dictionary! {
                "Filter" => dictionary! { "Name" => "Adobe.PPKLite" },
            }
        };
        if let Some(name) = signer_info.name {
            sig_dict.set("Name", Object::string_literal(name));
        }
        if let Some(reason) = signer_info.reason {
            sig_dict.set("Reason", Object::string_literal(reason));
        }
        if let Some(contact_info) = signer_info.contact_info {
            sig_dict.set("ContactInfo", Object::string_literal(contact_info));
        }
        if let Some(location) = signer_info.location {
            sig_dict.set("Location", Object::string_literal(location));
        }
        if let Some(date) = signer_info.date {
            sig_dict.set("M", Object::string_literal(date));
        }
        self.doc.new_document.add_object(sig_dict)
    }

    fn add_sig_annot_obj(
        &mut self,
        ap_normal_id: ObjectId,
        sig_id: ObjectId,
        page_id: ObjectId,
    ) -> ObjectId {
        let rect = vec![
            SEAL_POS.0.into(),
            SEAL_POS.1.into(),
            (SEAL_POS.0 + SEAL_SIZE.0).into(),
            (SEAL_POS.1 + SEAL_SIZE.1).into(),
        ];
        let sig_annot = dictionary! {
            "Type" => "Annot",
            "Subtype" => "Widget",
            "FT" => "Sig",
            "Rect" => rect,
            "T" => Object::string_literal("Signature1"),
            "V" => sig_id,
            "F" => 4,
            "P" => page_id,
            "AP" => dictionary! {
                "N" => ap_normal_id,
            },
        };
        self.doc.new_document.add_object(sig_annot)
    }

    fn clone_root(&mut self) -> Result<ObjectId> {
        let root_id = self
            .doc
            .get_prev_documents()
            .trailer
            .get(b"Root")?
            .as_reference()?;
        self.doc.opt_clone_object_to_new_document(root_id)?;
        Ok(root_id)
    }

    fn get_or_create_acro_form_mut(&mut self) -> Result<AcroForm> {
        self.clone_root()?;
        let acro_id = match self.doc.new_document.catalog()?.get(b"AcroForm") {
            Ok(acro_id) => {
                let acro_id = acro_id.as_reference()?;
                self.doc.opt_clone_object_to_new_document(acro_id)?;
                acro_id
            }
            Err(_) => {
                let acro_dict = dictionary! {
                    "Fields" => Object::Array(vec![]),
                };
                let acro_id = self.doc.new_document.add_object(acro_dict);
                self.doc
                    .new_document
                    .catalog_mut()?
                    .set("AcroForm", acro_id);
                acro_id
            }
        };
        let acro_form = self.doc.new_document.get_dictionary_mut(acro_id)?;
        Ok(AcroForm::new(acro_form))
    }

    fn get_page_mut(&mut self, page_id: ObjectId) -> Result<PageMut> {
        self.doc.opt_clone_object_to_new_document(page_id)?;
        let page = self.doc.new_document.get_dictionary_mut(page_id)?;
        Ok(PageMut::new(page))
    }

    fn get_page(&self, page_id: ObjectId) -> Result<Page> {
        let page = self.doc.get_prev_documents().get_dictionary(page_id)?;
        Ok(Page::new(page))
    }

    fn add_image_to_page(
        &mut self,
        page_id: ObjectId,
        mut rgb: Stream,
        alpha: Stream,
        pos: (f32, f32),
        size: (f32, f32),
    ) -> Result<()> {
        let alpha_id = self.doc.new_document.add_object(alpha);
        rgb.dict.set("SMask", alpha_id);
        let img_id = self.doc.new_document.add_object(rgb);
        let img_name = format!("X{}", img_id.0);
        self.doc.add_xobject(page_id, img_name.as_bytes(), img_id)?;

        let matrix = vec![
            size.0.into(),
            0.into(),
            0.into(),
            size.1.into(),
            pos.0.into(),
            pos.1.into(),
        ];
        let content = Content {
            operations: vec![
                Operation::new("q", vec![]),
                Operation::new("cm", matrix),
                Operation::new("Do", vec![img_name.into()]),
                Operation::new("Q", vec![]),
            ],
        };
        self.doc
            .new_document
            .add_to_page_content(page_id, content)?;
        Ok(())
    }

    fn add_cross_page_seal(&mut self) -> Result<()> {
        let page_ids: Vec<ObjectId> = self.doc.get_prev_documents().page_iter().collect();
        let page_ids_len = page_ids.len();
        let mut image_helper = ImageHelper::load_and_split("files/seal.png", page_ids_len)?;

        for (i, page_id) in page_ids.into_iter().enumerate() {
            let (rgb, alpha) = image_helper.get_img_pair(Some(i))?;

            // Calculate the position of the seal
            let page_size = self
                .get_page(page_id)?
                .get_box()?
                .into_iter()
                .map(|n| n as f32)
                .collect::<Vec<_>>();
            let height_center = (page_size[1] + page_size[3]) / 2.0;
            let size = (SEAL_SIZE.0 as f32 / page_ids_len as f32, SEAL_SIZE.1 as f32);
            let pos = (page_size[2] - size.0, height_center - size.1 / 2.0);

            self.add_image_to_page(page_id, rgb, alpha, pos, size)?;
        }
        Ok(())
    }

    fn add_ap_normal(&mut self) -> Result<ObjectId> {
        let mut image_helper = ImageHelper::load("files/seal.png")?;
        let (mut rgb, alpha) = image_helper.get_img_pair(None)?;
        let alpha_id = self.doc.new_document.add_object(alpha);
        rgb.dict.set("SMask", alpha_id);
        let img_id = self.doc.new_document.add_object(rgb);
        let ap_n_dict = dictionary!(
            "Type" => "XObject",
            "Subtype" => "Form",
            "FormType" => 1,
            "BBox" => vec![0, 0, SEAL_SIZE.0, SEAL_SIZE.1]
                .into_iter()
                .map(Object::Integer)
                .collect::<Vec<_>>(),
            "Resources" => dictionary! { "XObject" => dictionary! { "Image" => img_id } },
        );
        let ap_n_content = Content {
            operations: vec![
                Operation::new("q", vec![]),
                Operation::new(
                    "cm",
                    vec![
                        SEAL_SIZE.0.into(),
                        0.into(),
                        0.into(),
                        SEAL_SIZE.1.into(),
                        0.into(),
                        0.into(),
                    ],
                ),
                Operation::new("Do", vec!["Image".into()]),
                Operation::new("Q", vec![]),
            ],
        };
        let ap_n_stream = Stream::new(ap_n_dict, ap_n_content.encode()?);
        Ok(self.doc.new_document.add_object(ap_n_stream))
    }

    fn add_placeholder(&mut self, page_id: ObjectId, signer_info: SignerInfo) -> Result<()> {
        self.add_cross_page_seal()?;
        let sig_id = self.add_sig_obj(signer_info);
        let ap_normal_id = self.add_ap_normal()?;
        let sig_annot_id = self.add_sig_annot_obj(ap_normal_id, sig_id, page_id);
        let mut page = self.get_page_mut(page_id)?;
        let annots = page.get_or_create_annots_mut()?;
        annots.push(sig_annot_id.into());
        let mut acro_form = self.get_or_create_acro_form_mut()?;
        acro_form.set(b"SigFlags", Object::Integer(3));
        let fields = acro_form.get_fields_mut()?;
        fields.push(sig_annot_id.into());
        Ok(())
    }

    fn clone_sig_page(&mut self) -> Result<ObjectId> {
        let page_id = self
            .doc
            .get_prev_documents()
            .page_iter()
            .last()
            .ok_or_else(|| anyhow::anyhow!("No pages found in the document"))?;
        self.doc.opt_clone_object_to_new_document(page_id)?;
        Ok(page_id)
    }

    pub fn sign(&mut self, signer_info: SignerInfo) -> Result<()> {
        let page_id = self.clone_sig_page()?;
        self.add_placeholder(page_id, signer_info)?;

        let mut buffer = Vec::new();
        self.doc.save_to(&mut buffer)?;
        self.raw_pdf.load_data(buffer)?;

        self.raw_pdf.sign(self.signer.as_ref())?;

        Ok(())
    }

    #[inline]
    pub fn save_to<W: Write>(&mut self, target: &mut W) -> Result<()> {
        if !self.raw_pdf.is_empty() {
            self.raw_pdf.save_to(target)?;
        } else {
            self.doc.save_to(target)?;
        }
        Ok(())
    }

    #[inline]
    pub fn save(&mut self, path: &str) -> Result<()> {
        let mut file = std::fs::File::create(path)?;
        self.save_to(&mut file)?;
        Ok(())
    }
}
