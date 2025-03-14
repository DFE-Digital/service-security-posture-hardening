use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default)]
pub(crate) struct AdoMetadataBuilder<U, T, R>
where
    U: UrlType,
    T: TypeType,
    R: RestDocsType,
{
    url: U,
    organization: Option<String>,
    project: Option<String>,
    repo: Option<String>,
    //pub(crate) status: u16,
    // source: String,
    // sourcetype: String,
    tenant: Option<String>,
    r#type: T,
    rest_docs: R,
}

pub(crate) trait UrlType {}

#[derive(Default, Clone)]
pub(crate) struct NoUrl;
impl UrlType for NoUrl {}

#[derive(Default, Clone)]
pub(crate) struct SetUrl(String);
impl UrlType for SetUrl {}

#[derive(Default, Clone)]
pub(crate) struct NoType;

#[derive(Default, Clone)]
pub(crate) struct SetType(String);

pub(crate) trait TypeType {}
impl TypeType for NoType {}
impl TypeType for SetType {}

#[derive(Default, Clone)]
pub(crate) struct NoRestDocs;

#[derive(Clone)]
pub(crate) struct SetRestDocs(String);

pub(crate) trait RestDocsType {}
impl RestDocsType for NoRestDocs {}
impl RestDocsType for SetRestDocs {}

//<U: UrlType, O: OrganizationType, T: TypeType, R: RestDocsType>
impl AdoMetadataBuilder<NoUrl, NoType, NoRestDocs> {
    pub(crate) fn new() -> Self {
        AdoMetadataBuilder {
            url: NoUrl,
            organization: None,
            project: None,
            repo: None,
            tenant: None,
            r#type: NoType,
            rest_docs: NoRestDocs,
        }
    }
}

impl<U, T, R> AdoMetadataBuilder<U, T, R>
where
    U: UrlType,
    T: TypeType,
    R: RestDocsType,
{
    pub(crate) fn url<S: Into<String>>(self, url: S) -> AdoMetadataBuilder<SetUrl, T, R> {
        AdoMetadataBuilder {
            url: SetUrl(url.into()),
            ..self
        }
    }

    pub(crate) fn organization<S: Into<String>>(
        self,
        organization: S,
    ) -> AdoMetadataBuilder<U, T, R> {
        AdoMetadataBuilder {
            organization: Some(organization.into()),
            ..self
        }
    }

    pub(crate) fn project<S: Into<String>>(self, project: S) -> Self {
        Self {
            project: Some(project.into()),
            ..self
        }
    }

    pub(crate) fn repo<S: Into<String>>(self, repo: S) -> Self {
        Self {
            repo: Some(repo.into()),
            ..self
        }
    }

    pub(crate) fn tenant<S: Into<String>>(self, tenant: S) -> Self {
        Self {
            tenant: Some(tenant.into()),
            ..self
        }
    }

    pub(crate) fn r#type<S: Into<String>>(self, r#type: S) -> AdoMetadataBuilder<U, SetType, R> {
        AdoMetadataBuilder {
            r#type: SetType(r#type.into()),
            ..self
        }
    }

    pub(crate) fn rest_docs<S: Into<String>>(
        self,
        rest_docs: S,
    ) -> AdoMetadataBuilder<U, T, SetRestDocs> {
        AdoMetadataBuilder {
            rest_docs: SetRestDocs(rest_docs.into()),
            ..self
        }
    }
}

impl AdoMetadataBuilder<SetUrl, SetType, SetRestDocs> {
    pub(crate) fn build(self) -> AdoMetadata {
        let source = format!(
            "{}:{}",
            self.tenant.as_deref().unwrap_or("NO_TENANT"),
            self.url.0.as_str()
        );
        AdoMetadata {
            source,
            url: self.url.0,
            organization: self.organization,
            project: self.project,
            repo: self.repo,
            status: vec![],
            sourcetype: "ADO".into(),
            tenant: self.tenant,
            r#type: self.r#type.0,
            rest_docs: self.rest_docs.0,
        }
    }
}

#[derive(Default, Debug, Deserialize, Serialize, Clone)]
pub(crate) struct AdoMetadata {
    pub(crate) url: String,
    pub(crate) organization: Option<String>,
    pub(crate) project: Option<String>,
    pub(crate) repo: Option<String>,
    pub(crate) status: Vec<u16>,
    pub(crate) source: String,
    pub(crate) sourcetype: String,
    pub(crate) tenant: Option<String>,
    pub(crate) r#type: String,
    pub(crate) rest_docs: String,
}

pub(crate) trait AdoMetadataTrait {
    #[allow(unused)]
    fn set_metadata(&mut self, metadata: AdoMetadata);
    fn metadata(&self) -> Option<&AdoMetadata>;
    fn metadata_source(&self) -> &str {
        self.metadata()
            .map(|metadata| metadata.source.as_str())
            .unwrap_or("NO SOURCE FROM METADATA")
    }
    fn metadata_sourcetype(&self) -> &str {
        self.metadata()
            .map(|metadata| metadata.source.as_str())
            .unwrap_or("NO SOURCE FROM METADATA")
    }
}

impl AdoMetadata {
    pub(crate) fn url(&self) -> &str {
        &self.url
    }
}
