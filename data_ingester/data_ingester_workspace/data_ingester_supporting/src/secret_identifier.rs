use anyhow::Result;

/// Extract information about a secret from the keyvault secret ID.
/// format is "class--name--type"
/// where:
/// 'class' is the processor for the secret
/// 'name' is an identifier used to group distinct parts of a secret
/// 'token_type' the identifier for the part of the secret
#[derive(Debug)]
pub(crate) struct SecretIdentifier {
    pub(crate) id: String,
    pub(crate) _class: String,
    pub(crate) name: String,
    pub(crate) token_type: String,
}

impl SecretIdentifier {
    pub(crate) fn from_str(id: &str) -> Result<Self> {
        let mut iter = id.split("--");
        let secret_class = iter.next().and_then(|class| class.rsplit("/").next());
        let secret_name = iter.next();
        let secret_type = iter.next();
        let id = id.rsplit("/").next();
        match (id, secret_class, secret_name, secret_type) {
            (Some(id), Some(class), Some(name), Some(type_)) => Ok(Self {
                id: id.into(),
                _class: class.into(),
                name: name.into(),
                token_type: type_.into(),
            }),
            _ => anyhow::bail!("invalid format for secret identifier"),
        }
    }
}

#[cfg(test)]
mod test {
    use super::SecretIdentifier;

    #[test]
    fn test_secret_identifiers_from_str() {
        let example = "https://test.vault.azure.net/secrets/azure-dev-ops--foo--org";
        let secret_id = SecretIdentifier::from_str(example).expect("To parse");
        assert_eq!(secret_id.id, "azure-dev-ops--foo--org");
        assert_eq!(secret_id._class, "azure-dev-ops");
        assert_eq!(secret_id.name, "foo");
        assert_eq!(secret_id.token_type, "org");
    }

    #[test]
    fn test_secret_identifiers_from_str_with_hyphen() {
        let example = "https://test.vault.azure.net/secrets/azure-dev-ops--foo-bar-baz--org"; //azure-dev-ops--foo-bar-baz--org";
        let secret_id = SecretIdentifier::from_str(example).expect("To parse");
        assert_eq!(secret_id.id, "azure-dev-ops--foo-bar-baz--org");
        assert_eq!(secret_id._class, "azure-dev-ops");
        assert_eq!(secret_id.name, "foo-bar-baz");
        assert_eq!(secret_id.token_type, "org");
    }
}
