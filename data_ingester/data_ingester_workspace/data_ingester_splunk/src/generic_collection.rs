use serde::Serialize;

use crate::splunk::ToHecEvents;

pub struct GenericCollectionToSplunk<T: Serialize> {
    pub(crate) collection: Vec<T>,
    pub(crate) source: String,
    pub(crate) sourcetype: String,
    pub(crate) ssphp_run_key: String,
}

impl<T: Serialize> ToHecEvents for &GenericCollectionToSplunk<T> {
    type Item = T;

    fn source(&self) -> &str {
        self.source.as_str()
    }

    fn sourcetype(&self) -> &str {
        self.sourcetype.as_str()
    }

    fn collection<'i>(&'i self) -> Box<dyn Iterator<Item = &'i Self::Item> + 'i> {
        Box::new(self.collection.iter())
    }

    fn ssphp_run_key(&self) -> &str {
        self.ssphp_run_key.as_str()
    }
}

impl<T: Serialize> ToHecEvents for GenericCollectionToSplunk<T> {
    type Item = T;

    fn source(&self) -> &str {
        self.source.as_str()
    }

    fn sourcetype(&self) -> &str {
        self.sourcetype.as_str()
    }

    fn collection<'i>(&'i self) -> Box<dyn Iterator<Item = &'i Self::Item> + 'i> {
        Box::new(self.collection.iter())
    }

    fn ssphp_run_key(&self) -> &str {
        self.ssphp_run_key.as_str()
    }
}
