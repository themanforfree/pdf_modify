#[derive(Debug, Clone, Default)]
pub struct SignerInfo {
    pub name: Option<String>,
    pub reason: Option<String>,
    pub contact_info: Option<String>,
    pub location: Option<String>,
    pub date: Option<String>,
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

    pub fn builder() -> SignerInfoBuilder {
        SignerInfoBuilder::default()
    }
}

#[derive(Debug, Clone, Default)]
pub struct SignerInfoBuilder {
    name: Option<String>,
    reason: Option<String>,
    contact_info: Option<String>,
    location: Option<String>,
    date: Option<String>,
}

impl SignerInfoBuilder {
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    pub fn reason(mut self, reason: impl Into<String>) -> Self {
        self.reason = Some(reason.into());
        self
    }

    pub fn contact_info(mut self, contact_info: impl Into<String>) -> Self {
        self.contact_info = Some(contact_info.into());
        self
    }

    pub fn location(mut self, location: impl Into<String>) -> Self {
        self.location = Some(location.into());
        self
    }

    pub fn date(mut self, date: impl Into<String>) -> Self {
        self.date = Some(date.into());
        self
    }

    pub fn build(self) -> SignerInfo {
        SignerInfo {
            name: self.name,
            reason: self.reason,
            contact_info: self.contact_info,
            location: self.location,
            date: self.date,
        }
    }
}
