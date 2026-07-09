use anyhow::Result;
use hickory_proto::rr::RecordType;
use hickory_proto::rr::RData;
use hickory_resolver::config::*;
use hickory_resolver::net::runtime::TokioRuntimeProvider;
use hickory_resolver::TokioResolver;

pub async fn resolve_txt_record<T: AsRef<str>>(domain: T) -> Result<Vec<String>> {
    let resolver = TokioResolver::builder_with_config(
        ResolverConfig::default(),
        TokioRuntimeProvider::default(),
    )
    .build()?;

    // Lookup the TXT record associated with a name.
    let response = resolver.lookup(domain.as_ref(), RecordType::TXT).await?;
    let txts = response
        .answers()
        .iter()
        .filter_map(|record| match &record.data {
            RData::TXT(txt) => Some(txt.to_string()),
            _ => None,
        })
        .collect::<Vec<String>>();

    Ok(txts)
}

#[cfg(feature = "live_tests")]
#[cfg(test)]
mod test {
    use crate::dns::resolve_txt_record;
    use anyhow::Result;

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
