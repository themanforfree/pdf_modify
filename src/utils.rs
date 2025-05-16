use anyhow::Result;
use lopdf::{Dictionary, Object};

pub(crate) struct Page<'a> {
    dict: &'a mut Dictionary,
}

impl<'a> Page<'a> {
    pub(crate) fn new(dict: &'a mut Dictionary) -> Self {
        Page { dict }
    }

    pub(crate) fn get_or_create_annots_mut(&mut self) -> Result<&mut Vec<Object>> {
        self.dict
            .as_hashmap_mut()
            .entry(b"Annots".into())
            .or_insert_with(|| Object::Array(vec![]))
            .as_array_mut()
            .map_err(Into::into)
    }
}

pub(crate) struct AcroForm<'a> {
    dict: &'a mut Dictionary,
}

impl<'a> AcroForm<'a> {
    pub(crate) fn new(dict: &'a mut Dictionary) -> Self {
        AcroForm { dict }
    }

    pub(crate) fn get_fields_mut(&mut self) -> Result<&mut Vec<Object>> {
        self.dict
            .get_mut(b"Fields")
            .and_then(Object::as_array_mut)
            .map_err(Into::into)
    }

    pub(crate) fn set<K, V>(&mut self, key: K, value: V)
    where
        K: Into<Vec<u8>>,
        V: Into<Object>,
    {
        self.dict.set(key, value);
    }
}
