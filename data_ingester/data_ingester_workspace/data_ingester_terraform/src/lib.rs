use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fs, path::Path};

/// The top level structure representing a Terraform state
/// Resources can be filtered using the resources method
///
/// ```
/// use data_ingester_terraform::State;
/// let state = State::from_path_str("test/terraform.tfstate").unwrap();
/// let github_resources = state.resources()
///                             .by_provider("github")
///                             .by_type("github_user")
///                             .by_name("gh_user")
///                             .filter();
/// ```
#[derive(Serialize, Deserialize, Debug)]
pub struct State {
    version: i32,
    terraform_version: String,
    serial: i32,
    lineage: String,
    resources: Vec<Resource>,
}

impl State {
    /// Load a Terraform state file form a Path
    pub fn from_path_str<P: AsRef<Path>>(path: P) -> Result<Self> {
        let state_json =
            fs::read_to_string(path).expect("Something went wrong reading the state file");

        serde_json::from_str(&state_json).map_err(|err| err.into())
    }

    /// Access resources with filtering
    pub fn resources(&self) -> ResourcesFilter {
        ResourcesFilter {
            inner: self.resources.as_slice(),
            provider: vec![],
            name: vec![],
            r#type: vec![],
            filter_operation: ResourceFilterOperation::And,
        }
    }
}

/// Filter Terraform resources
/// Filters are additive, the retured Resources must pass all supplied filters
/// Multiple filters can be applied to the resources
#[derive(Debug)]
pub struct ResourcesFilter<'a> {
    pub inner: &'a [Resource],
    provider: Vec<String>,
    name: Vec<String>,
    r#type: Vec<String>,
    filter_operation: ResourceFilterOperation,
}

/// The combinator effect for multiple filters
#[derive(Debug)]
pub enum ResourceFilterOperation {
    And,
    Or,
}

impl<'a> ResourcesFilter<'a> {
    pub fn filter(self) -> Vec<&'a Resource> {
        self.inner
            .iter()
            .filter(|resource| {
                let mut results = vec![];

                self.provider.iter().for_each(|provider| {
                    results.push(resource.provider.contains(provider));
                });

                self.r#type.iter().for_each(|ty| {
                    results.push(resource.r#type.contains(ty));
                });

                self.name.iter().for_each(|name| {
                    results.push(resource.name.contains(name));
                });

                match self.filter_operation {
                    ResourceFilterOperation::And => results.iter().all(|result| *result),
                    ResourceFilterOperation::Or => results.iter().any(|result| *result),
                }
            })
            .collect()
    }

    /// Filter Terrafrom resources by provider
    pub fn by_provider<S: Into<String>>(mut self, provider: S) -> Self {
        self.provider.push(provider.into());
        self
    }

    /// Filter Terrafrom resources by type
    pub fn by_type<S: Into<String>>(mut self, ty: S) -> Self {
        self.r#type.push(ty.into());
        self
    }

    /// Filter Terrafrom resources by name    
    pub fn by_name<S: Into<String>>(mut self, name: S) -> Self {
        self.name.push(name.into());
        self
    }

    /// Change the filter operation mode
    /// ```
    /// use data_ingester_terraform::{State, ResourceFilterOperation};
    /// let state = State::from_path_str("test/terraform.tfstate").unwrap();
    /// let resources = state.resources()
    ///                             .set_filter_operation(ResourceFilterOperation::Or)        
    ///                             .by_provider("hashicorp/random")
    ///                             .by_name("gh_user")            
    ///                             .filter();
    /// assert_eq!(resources.len(), 2);
    /// ```
    pub fn set_filter_operation(mut self, filter_operation: ResourceFilterOperation) -> Self {
        self.filter_operation = filter_operation;
        self
    }
}

/// A single Terraform resource
#[derive(Serialize, Deserialize, Debug)]
pub struct Resource {
    mode: String,
    r#type: String,
    name: String,
    provider: String,
    instances: Vec<Instance>,
}

/// A Terraform Resource's Instance state
#[derive(Serialize, Deserialize, Debug)]
pub struct Instance {
    schema_version: i32,
    attributes: HashMap<String, serde_json::Value>,
    sensitive_attributes: Vec<()>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state_path() -> &'static str {
        "test/terraform.tfstate"
    }

    /// Test all top level properties parse
    #[test]
    fn top_level_properties() -> Result<()> {
        let state = State::from_path_str(state_path())?;
        assert_eq!(state.version, 4);
        assert_eq!(state.terraform_version, "1.9.5");
        assert_eq!(state.serial, 77);
        assert_eq!(state.lineage, "fee5af10-2677-c045-72dd-5c7b4c4f924e");
        Ok(())
    }

    #[test]
    fn resources() -> Result<()> {
        let state = State::from_path_str(state_path())?;
        assert!(!state.resources.is_empty());
        Ok(())
    }

    #[test]
    fn resources_by_provider() -> Result<()> {
        let state = State::from_path_str(state_path())?;
        let github_resources = state.resources().by_provider("github").filter();
        assert!(!github_resources.is_empty());
        github_resources.iter().for_each(|resource| {
            assert!(resource.provider.contains("github"));
        });
        Ok(())
    }

    #[test]
    fn resources_by_name() -> Result<()> {
        let state = State::from_path_str(state_path())?;
        let github_resources = state.resources().by_name("gh_user").filter();
        assert!(!github_resources.is_empty());
        github_resources.iter().for_each(|resource| {
            assert!(resource.name.contains("gh_user"));
        });
        Ok(())
    }

    #[test]
    fn resources_by_type() -> Result<()> {
        let state = State::from_path_str(state_path())?;
        let github_resources = state.resources().by_type("github_user").filter();
        assert!(!github_resources.is_empty());
        dbg!(&github_resources);
        github_resources.iter().for_each(|resource| {
            assert!(resource.r#type.contains("github_user"));
        });
        Ok(())
    }

    #[test]
    fn resources_filter_chaining() -> Result<()> {
        let state = State::from_path_str(state_path())?;
        let github_resources = state
            .resources()
            .by_provider("github")
            .by_type("github_user")
            .by_name("gh_user")
            .filter();
        assert_eq!(github_resources.len(), 1);
        github_resources.iter().for_each(|resource| {
            assert!(resource.provider.contains("github"));
            assert!(resource.r#type.contains("github_user"));
            assert!(resource.name.contains("gh_user"));
        });
        Ok(())
    }

    #[test]
    fn resources_filter_chaining_or() -> Result<()> {
        let state = State::from_path_str(state_path())?;
        let github_resources = state
            .resources()
            .set_filter_operation(ResourceFilterOperation::Or)
            .by_provider("random")
            .by_name("gh_user")
            .filter();
        assert_eq!(github_resources.len(), 2);
        github_resources.iter().for_each(|resource| {
            assert!(resource.provider.contains("random") || resource.name.contains("gh_user"));
        });
        Ok(())
    }
}
