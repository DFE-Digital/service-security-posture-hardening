use anyhow::Result;
use hickory_proto::rr::record_type::RecordType;
use hickory_resolver::config::*;
use hickory_resolver::TokioResolver;

pub async fn resolve_txt_record<T: AsRef<str>>(domain: T) -> Result<Vec<String>> {
    // Construct a new Resolver with default configuration options
    let resolver = TokioResolver::tokio(ResolverConfig::default(), ResolverOpts::default());

    // Lookup the TXT record associated with a name.
    let response = resolver.lookup(domain.as_ref(), RecordType::TXT).await?;
    let txts = response
        .iter()
        .filter_map(|r| r.as_txt())
        .map(|r| r.to_string())
        .collect::<Vec<String>>();

    Ok(txts)
}

#[cfg(feature = "live_tests")]
#[cfg(test)]
mod test {
    use crate::dns::resolve_txt_record;

    #[tokio::test]
    async fn test_resolve_txt_record() -> Result<()> {
        let result = resolve_txt_record("www.gmail.com").await?;
        assert!(!result.is_empty());
        for txt in result {
            assert!(txt.contains("google-site-verification"));
        }
        Ok(())
    }
}
