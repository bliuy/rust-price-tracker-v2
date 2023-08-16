pub mod requests {
    include!(concat!(env!("OUT_DIR"), "/scraping.requests.rs"));
}
pub mod results {
    include!(concat!(env!("OUT_DIR"), "/scraping.results.rs"));
}

pub mod json_results {
    use std::collections::HashMap;

    use super::results::ScrapingResult;

    #[derive(Debug, serde::Deserialize, serde::Serialize)]
    pub struct ScrapingResultJson {
        source: String,
        utc_timestamp: u64,
        name: String,
        identifier: String,
        price: f32,
        attributes: HashMap<String, String>,
        metadata: HashMap<String, String>,
    }

    impl From<ScrapingResult> for ScrapingResultJson {
        fn from(value: ScrapingResult) -> Self {
            ScrapingResultJson {
                source: value.source,
                utc_timestamp: value.utc_timestamp,
                name: value.name,
                identifier: value.identifier,
                price: value.price,
                attributes: value.attributes,
                metadata: value.metadata,
            }
        }
    }
}
