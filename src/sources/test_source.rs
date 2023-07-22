use crate::{scraping, scraping_traits};

impl scraping_traits::Source for scraping::requests::Test {
    fn get_source_name(&self) -> String {
        "test".to_string()
    }
}
