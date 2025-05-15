use anyhow::Result;
use lopdf::{Dictionary, IncrementalDocument, Object, ObjectId, StringFormat, dictionary, xobject};

use crate::{
    config::SIG_CONTENTS_PLACEHOLDER_LEN,
    parser::RawPdf,
    signer::{P12Signer, Sign},
};

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
        // .add_object(Stream::new(sig_dict, vec![]))
    }

    fn add_sig_annot_obj(&mut self, sig_id: ObjectId, page_id: ObjectId) -> ObjectId {
        let boundingbox = vec![0.0, 0.0, 0.0, 0.0];
        let matrix = vec![1.0, 0.0, 0.0, 1.0, 0.0, 0.0];
        let ap_stream = xobject::form(boundingbox, matrix, vec![]);
        let ap_id = self.doc.new_document.add_object(ap_stream);
        let sig_annot = dictionary! {
            "Type" => "Annot",
            "Subtype" => "Widget",
            "FT" => "Sig",
            "Rect" => vec![0, 0, 0, 0].into_iter().map(Object::Integer).collect::<Vec<_>>(),
            "T" => Object::string_literal("Signature1"),
            "V" => sig_id,
            "F" => 4,
            "P" => page_id,
            "AP" => dictionary! {
                "N" => ap_id,
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
        Ok(AcroForm { dict: acro_form })
    }

    fn get_page_mut(&mut self, page_id: ObjectId) -> Result<Page> {
        self.doc.opt_clone_object_to_new_document(page_id)?;
        let page = self.doc.new_document.get_dictionary_mut(page_id)?;
        Ok(Page { dict: page })
    }

    fn add_placeholder(&mut self, page_id: ObjectId, signer_info: SignerInfo) -> Result<()> {
        let sig_id = self.add_sig_obj(signer_info);
        let sig_annot_id = self.add_sig_annot_obj(sig_id, page_id);
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
            .next()
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

    // #[inline]
    // pub fn save_to<W: Write>(&mut self, target: &mut W) -> Result<()> {
    //     if !self.raw_pdf.is_empty() {
    //         self.raw_pdf.save_to(target)?;
    //     } else {
    //         self.doc.save_to(target)?;
    //     }
    //     Ok(())
    // }

    #[inline]
    pub fn save(&mut self, path: &str) -> Result<()> {
        // let mut file = std::fs::File::create(path)?;
        // self.save_to(&mut file)?;
        self.raw_pdf.save(path)?;
        Ok(())
    }
}

struct Page<'a> {
    dict: &'a mut Dictionary,
}

impl Page<'_> {
    fn get_or_create_annots_mut(&mut self) -> Result<&mut Vec<Object>> {
        self.dict
            .as_hashmap_mut()
            .entry(b"Annots".into())
            .or_insert_with(|| Object::Array(vec![]))
            .as_array_mut()
            .map_err(Into::into)
    }
}

struct AcroForm<'a> {
    dict: &'a mut Dictionary,
}

impl AcroForm<'_> {
    fn get_fields_mut(&mut self) -> Result<&mut Vec<Object>> {
        self.dict
            .get_mut(b"Fields")
            .and_then(Object::as_array_mut)
            .map_err(Into::into)
    }

    pub fn set<K, V>(&mut self, key: K, value: V)
    where
        K: Into<Vec<u8>>,
        V: Into<Object>,
    {
        self.dict.set(key, value);
    }
}

pub struct SignerInfo {
    name: Option<String>,
    reason: Option<String>,
    contact_info: Option<String>,
    location: Option<String>,
    date: Option<String>,
}

impl SignerInfo {
    pub fn new(
        name: impl Into<String>,
        reason: impl Into<String>,
        contact_info: impl Into<String>,
        location: impl Into<String>,
        date: impl Into<String>,
    ) -> Self {
        SignerInfo {
            name: Some(name.into()),
            reason: Some(reason.into()),
            contact_info: Some(contact_info.into()),
            location: Some(location.into()),
            date: Some(date.into()),
        }
    }
    pub fn empty() -> Self {
        SignerInfo {
            name: None,
            reason: None,
            contact_info: None,
            location: None,
            date: None,
        }
    }
}
