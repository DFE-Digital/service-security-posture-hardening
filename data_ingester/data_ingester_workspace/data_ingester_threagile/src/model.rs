use anyhow::Result;
use serde::Deserialize;
use serde::Serialize;
use serde_with::serde_as;
use serde_with::DisplayFromStr;
use std::collections::HashMap;
use std::fs::File;
use std::io::prelude::*;

#[derive(Serialize, Debug)]
#[serde(rename_all = "snake_case")]
pub(crate) struct Model {
    threagile_version: String,
    title: String,
    business_criticality: TechnicalAssetCriticality,
    pub(crate) technical_assets: TechnicalAssets,
}

#[derive(Serialize, Default, Debug)]
#[serde(rename_all = "snake_case")]
#[serde(transparent)]
pub(crate) struct TechnicalAssets(pub(crate) HashMap<String, TechnicalAsset>);

#[serde_as]
#[derive(Serialize, Debug)]
pub(crate) struct TechnicalAsset {
    id: String,
    description: String,
    #[serde_as(as = "DisplayFromStr")]
    usage: TechnicalAssetUsage,
    #[serde_as(as = "DisplayFromStr")]
    r#type: TechnicalAssetType,
    #[serde_as(as = "DisplayFromStr")]
    size: TechnicalAssetSize,
    #[serde_as(as = "DisplayFromStr")]
    encryption: TechnicalAssetEncryption,
    #[serde_as(as = "DisplayFromStr")]
    machine: TechnicalAssetMachine,
    #[serde_as(as = "DisplayFromStr")]
    confidentiality: TechnicalAssetConfidentiality,
    #[serde_as(as = "DisplayFromStr")]
    integrity: TechnicalAssetCriticality,
    #[serde_as(as = "DisplayFromStr")]
    availability: TechnicalAssetCriticality,
    out_of_scope: bool,
    #[serde_as(as = "DisplayFromStr")]
    technology: Technology,
    used_as_client_by_human: bool,
    internet: bool,
    multi_tenant: bool,
    redundant: bool,
    custom_developed_parts: bool,
}

impl Default for TechnicalAsset {
    fn default() -> Self {
        Self {
            id: Default::default(),
            description: Default::default(),
            usage: Default::default(),
            r#type: Default::default(),
            size: Default::default(),
            encryption: Default::default(),
            machine: Default::default(),
            confidentiality: Default::default(),
            integrity: Default::default(),
            availability: TechnicalAssetCriticality::Important,
            out_of_scope: Default::default(),
            technology: Default::default(),
            used_as_client_by_human: Default::default(),
            internet: true,
            multi_tenant: Default::default(),
            redundant: Default::default(),
            custom_developed_parts: Default::default(),
        }
    }
}

#[derive(Default, Debug, Serialize)]
#[allow(dead_code)]
enum Technology {
    #[default]
    #[serde(rename = "AI")]
    Ai,
    ApplicationServer,
    ArtifactRegistry,
    BatchProcessing,
    BigDataPlatform,
    BlockStorage,
    Browser,
    BuildPipeline,
    ClientSystem,
    #[serde(rename = "CLI")]
    Cli,
    #[serde(rename = "CMS")]
    Cms,
    CodeInspectionPlatform,
    ContainerPlatform,
    DataLake,
    Database,
    Desktop,
    DevOpsClient,
    #[serde(rename = "EJB")]
    Ejb,
    #[serde(rename = "ERP")]
    Erp,
    EventListener,
    FileServer,
    Function,
    Gateway,
    #[serde(rename = "HSM")]
    Hsm,
    IdentityProvider,
    IdentityStoreDatabase,
    IdentityStoreLDAP,
    #[serde(rename = "IDS")]
    Ids,
    IoTDevice,
    #[serde(rename = "IPS")]
    Ips,
    LDAPServer,
    Library,
    LoadBalancer,
    LocalFileSystem,
    MailServer,
    Mainframe,
    MessageQueue,
    MobileApp,
    Monitoring,
    ReportEngine,
    ReverseProxy,
    Scheduler,
    SearchEngine,
    SearchIndex,
    ServiceMesh,
    ServiceRegistry,
    SourcecodeRepository,
    StreamProcessing,
    Task,
    Tool,
    #[serde(rename = "UnknownTechnology")]
    Unknown,
    Vault,
    #[serde(rename = "WAF")]
    Waf,
    WebApplication,
    WebServer,
    WebServiceREST,
    WebServiceSOAP,
}

impl std::fmt::Display for Technology {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use Technology::*;
        match self {
            Ai => write!(f, "ai")?,
            ApplicationServer => write!(f, "application-server")?,
            ArtifactRegistry => write!(f, "artifact-registry")?,
            BatchProcessing => write!(f, "batch-processing")?,
            BigDataPlatform => write!(f, "big-data-platform")?,
            BlockStorage => write!(f, "block-storage")?,
            Browser => write!(f, "browser")?,
            BuildPipeline => write!(f, "build-pipeline")?,
            ClientSystem => write!(f, "client-system")?,
            Cli => write!(f, "cli")?,
            Cms => write!(f, "cms")?,
            CodeInspectionPlatform => write!(f, "code-inspection-platform")?,
            ContainerPlatform => write!(f, "container-platform")?,
            DataLake => write!(f, "data-lake")?,
            Database => write!(f, "database")?,
            Desktop => write!(f, "desktop")?,
            DevOpsClient => write!(f, "devops-client")?,
            Ejb => write!(f, "ejb")?,
            Erp => write!(f, "erp")?,
            EventListener => write!(f, "event-listener")?,
            FileServer => write!(f, "file-server")?,
            Function => write!(f, "function")?,
            Gateway => write!(f, "gateway")?,
            Hsm => write!(f, "hsm")?,
            IdentityProvider => write!(f, "identity-provider")?,
            IdentityStoreDatabase => write!(f, "identity-store-database")?,
            IdentityStoreLDAP => write!(f, "identity-store-ldap")?,
            Ids => write!(f, "ids")?,
            IoTDevice => write!(f, "iot-device")?,
            Ips => write!(f, "ips")?,
            LDAPServer => write!(f, "ldap-server")?,
            Library => write!(f, "library")?,
            LoadBalancer => write!(f, "load-balancer")?,
            LocalFileSystem => write!(f, "local-file-system")?,
            MailServer => write!(f, "mail-server")?,
            Mainframe => write!(f, "mainframe")?,
            MessageQueue => write!(f, "message-queue")?,
            MobileApp => write!(f, "mobile-app")?,
            Monitoring => write!(f, "monitoring")?,
            ReportEngine => write!(f, "report-engine")?,
            ReverseProxy => write!(f, "reverse-proxy")?,
            Scheduler => write!(f, "scheduler")?,
            SearchEngine => write!(f, "search-engine")?,
            SearchIndex => write!(f, "search-index")?,
            ServiceMesh => write!(f, "service-mesh")?,
            ServiceRegistry => write!(f, "service-registry")?,
            SourcecodeRepository => write!(f, "sourcecode-repository")?,
            StreamProcessing => write!(f, "stream-processing")?,
            Task => write!(f, "task")?,
            Tool => write!(f, "tool")?,
            Unknown => write!(f, "unknown-technology")?,
            Vault => write!(f, "vault")?,
            Waf => write!(f, "waf")?,
            WebApplication => write!(f, "web-application")?,
            WebServer => write!(f, "web-server")?,
            WebServiceREST => write!(f, "web-service-rest")?,
            WebServiceSOAP => write!(f, "web-service-soap")?,
        }
        Ok(())
    }
}

#[derive(Default, Debug, Serialize)]
#[allow(dead_code)]
enum TechnicalAssetUsage {
    #[default]
    Business,
    Devops,
}
impl std::fmt::Display for TechnicalAssetUsage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TechnicalAssetUsage::Business => write!(f, "business")?,
            TechnicalAssetUsage::Devops => write!(f, "devops")?,
        }
        Ok(())
    }
}

#[derive(Default, Debug, Serialize)]
#[allow(dead_code)]
enum TechnicalAssetCriticality {
    Archive,
    Operational,
    Important,
    #[default]
    Critical,
    MissionCritical,
}
impl std::fmt::Display for TechnicalAssetCriticality {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TechnicalAssetCriticality::Archive => write!(f, "archive")?,
            TechnicalAssetCriticality::Operational => write!(f, "operational")?,
            TechnicalAssetCriticality::Important => write!(f, "important")?,
            TechnicalAssetCriticality::Critical => write!(f, "critical")?,
            TechnicalAssetCriticality::MissionCritical => write!(f, "mission-critical")?,
        }
        Ok(())
    }
}

#[derive(Default, Debug, Serialize)]
#[allow(dead_code)]
enum TechnicalAssetConfidentiality {
    Public,
    Internal,
    Restricted,
    #[default]
    Confidential,
    StrictlyConfidential,
}

impl std::fmt::Display for TechnicalAssetConfidentiality {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TechnicalAssetConfidentiality::Public => write!(f, "public")?,
            TechnicalAssetConfidentiality::Internal => write!(f, "internal")?,
            TechnicalAssetConfidentiality::Restricted => write!(f, "restricted")?,
            TechnicalAssetConfidentiality::Confidential => write!(f, "confidential")?,
            TechnicalAssetConfidentiality::StrictlyConfidential => {
                write!(f, "strictly-confidential")?
            }
        }
        Ok(())
    }
}

#[derive(Default, Debug, Serialize)]
enum TechnicalAssetMachine {
    #[default]
    Physical,
    Virtual,
    Container,
    Serverless,
}

impl std::fmt::Display for TechnicalAssetMachine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TechnicalAssetMachine::Physical => write!(f, "physical")?,
            TechnicalAssetMachine::Virtual => write!(f, "virtual")?,
            TechnicalAssetMachine::Container => write!(f, "container")?,
            TechnicalAssetMachine::Serverless => write!(f, "serverless")?,
        }
        Ok(())
    }
}

#[derive(Default, Debug, Serialize)]
#[allow(dead_code)]
enum TechnicalAssetEncryption {
    #[default]
    None,
    Transparent,
    Symmetric,
    Asymmetric,
    Individual,
}

impl std::fmt::Display for TechnicalAssetEncryption {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TechnicalAssetEncryption::None => write!(f, "none")?,
            TechnicalAssetEncryption::Transparent => write!(f, "transparent")?,
            TechnicalAssetEncryption::Symmetric => write!(f, "data-with-symmetric-shared-key")?,
            TechnicalAssetEncryption::Asymmetric => write!(f, "data-with-asymmetric-shared-key")?,
            TechnicalAssetEncryption::Individual => write!(f, "data-with-end-user-individual-key")?,
        }
        Ok(())
    }
}

#[derive(Default, Debug, Serialize)]
#[allow(dead_code)]
enum TechnicalAssetType {
    #[default]
    ExternalEntity,
    Process,
    Datastore,
}

impl std::fmt::Display for TechnicalAssetType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TechnicalAssetType::ExternalEntity => write!(f, "external-entity")?,
            TechnicalAssetType::Process => write!(f, "process")?,
            TechnicalAssetType::Datastore => write!(f, "datastore")?,
        }
        Ok(())
    }
}

#[derive(Serialize, Default, Debug)]
#[allow(dead_code)]
enum TechnicalAssetSize {
    System,
    Service,
    Application,
    #[default]
    Component,
}

impl std::fmt::Display for TechnicalAssetSize {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TechnicalAssetSize::System => write!(f, "datastore")?,
            TechnicalAssetSize::Service => write!(f, "service")?,
            TechnicalAssetSize::Application => write!(f, "application")?,
            TechnicalAssetSize::Component => write!(f, "component")?,
        }
        Ok(())
    }
}

impl Model {
    // TODO DELETE BEFORE PR
    // fn default() -> Self {
    //     Model {
    //         threagile_version: "1.0.0".to_string(),
    //         title: "Results from splunk".to_string(),
    //         business_criticality: TechnicalAssetCriticality::Important,
    //         technical_assets: TechnicalAssets::default(),
    //     }
    // }

    // fn push_ta(&mut self, ta: TechnicalAsset) {
    //     let _ = self.technical_assets.0.insert(ta.id.to_string(), ta);
    // }

    pub(crate) fn write_file(self, filename: &str) -> Result<()> {
        let mut file = File::create(filename)?;
        let output = serde_yaml::to_string(&self)?;
        file.write_all(output.as_bytes())?;
        Ok(())
    }
}

impl Default for Model {
    fn default() -> Self {
        let mut technical_assets = HashMap::new();
        let _ = technical_assets.insert(
            "asset1".to_string(),
            TechnicalAsset {
                id: "asset1-id".to_string(),
                description: "this is a test".to_string(),
                usage: TechnicalAssetUsage::Business,
                r#type: TechnicalAssetType::ExternalEntity,
                size: TechnicalAssetSize::Component,
                encryption: TechnicalAssetEncryption::None,
                machine: TechnicalAssetMachine::Physical,
                confidentiality: TechnicalAssetConfidentiality::Public,
                integrity: TechnicalAssetCriticality::Archive,
                availability: TechnicalAssetCriticality::Archive,
                out_of_scope: false,
                technology: Technology::Ai,
                used_as_client_by_human: true,
                internet: true,
                multi_tenant: true,
                redundant: true,
                custom_developed_parts: true,
            },
        );
        Self {
            threagile_version: "1".to_string(),
            title: "test foo".to_string(),
            business_criticality: TechnicalAssetCriticality::Critical,
            technical_assets: TechnicalAssets(technical_assets),
        }
    }
}

impl From<SplunkResult> for TechnicalAsset {
    fn from(value: SplunkResult) -> Self {
        let ta = match value.r#type.as_str() {
            "microsoft.web/sites" => TechnicalAsset {
                id: value.resource_id.to_string(),
                size: TechnicalAssetSize::Service,
                machine: TechnicalAssetMachine::Serverless,
                technology: Technology::Function,
                ..Default::default()
            },
            "microsoft.app/containerapps" => TechnicalAsset {
                id: value.resource_id.to_string(),
                description: "A container app running a web application for the public."
                    .to_string(),
                size: TechnicalAssetSize::Service,
                machine: TechnicalAssetMachine::Container,
                technology: Technology::WebApplication,
                ..Default::default()
            },
            "microsoft.keyvault/vaults" => TechnicalAsset {
                id: value.resource_id.to_string(),
                description: "A key vault used to hold sensitive keys, secrets, and config."
                    .to_string(),
                size: TechnicalAssetSize::Service,
                machine: TechnicalAssetMachine::Virtual,
                technology: Technology::Vault,
                ..Default::default()
            },
            "microsoft.cache/redis" => TechnicalAsset {
                id: value.resource_id.to_string(),
                size: TechnicalAssetSize::Service,
                machine: TechnicalAssetMachine::Virtual,
                technology: Technology::Database,
                ..Default::default()
            },
            "microsoft.storage/storageaccounts" => TechnicalAsset {
                id: value.resource_id.to_string(),
                size: TechnicalAssetSize::Service,
                machine: TechnicalAssetMachine::Virtual,
                technology: Technology::BlockStorage,
                ..Default::default()
            },
            "microsoft.sql/servers/databases" => TechnicalAsset {
                id: value.resource_id.to_string(),
                size: TechnicalAssetSize::Service,
                machine: TechnicalAssetMachine::Virtual,
                technology: Technology::Database,
                ..Default::default()
            },
            "microsoft.compute/virtualmachines" => TechnicalAsset {
                id: value.resource_id.to_string(),
                size: TechnicalAssetSize::Service,
                machine: TechnicalAssetMachine::Virtual,
                technology: Technology::ApplicationServer,
                ..Default::default()
            },
            "microsoft.authorization/roleassignments" => TechnicalAsset {
                id: value.resource_id.to_string(),
                size: TechnicalAssetSize::Service,
                machine: TechnicalAssetMachine::Virtual,
                technology: Technology::IdentityStoreDatabase,
                ..Default::default()
            },
            _ => TechnicalAsset {
                id: value.resource_id.to_string(),
                ..Default::default()
            },
        };
        ta
    }
}

impl From<SplunkResults> for TechnicalAssets {
    fn from(value: SplunkResults) -> Self {
        let mut collection = HashMap::with_capacity(value.len());
        for result in value.results {
            let _ = collection.insert(result.resource_id.to_string(), result.into());
        }
        TechnicalAssets(collection)
    }
}

/// A type representing the fields returned from the Splunk search `ssphp_get_list_service_resources`
/// DCAP/default/savedsearches.conf.d/ssphp_use_case_general.conf.d/ssphp_get_list_service_resources.conf
///
#[allow(dead_code)]
#[derive(Deserialize, Default, Clone, Debug)]
pub(crate) struct SplunkResult {
    kind: String,
    #[serde(rename = "resourceGroup")]
    resource_group: String,
    pub(crate) resource_id: String,
    pub service_id: String,
    #[serde(rename = "SSPHP_RUN")]
    ssphp_run: String,
    r#type: String,
}

#[derive(Deserialize, Default, Debug)]
pub(crate) struct SplunkResults {
    results: Vec<SplunkResult>,
}

impl SplunkResults {
    fn len(&self) -> usize {
        self.results.len()
    }
}

#[cfg(test)]
mod test {
    use super::{Model, SplunkResult, SplunkResults, TechnicalAssets};
    use anyhow::Result;

    fn splunk_results() -> SplunkResults {
        SplunkResults {
            results: vec![SplunkResult {
                service_id: "service_id".to_string(),
                ssphp_run: "foo".to_string(),
                resource_id: "splunk-results-foo".to_string(),
                r#type: "microsoft.web/sites".to_string(),
                kind: "functionapp,linux".to_string(),
                resource_group: "rg1".to_string(),
            }],
        }
    }

    #[test]
    fn test_data() -> Result<()> {
        let model = Model::default();
        model.write_file("test1.yaml")?;
        Ok(())
    }

    #[test]
    fn test_from_splunk_result() -> Result<()> {
        let model = Model {
            technical_assets: TechnicalAssets::from(splunk_results()),
            ..Default::default()
        };

        model.write_file("results_from_splunk.yaml")?;
        Ok(())
    }
}
