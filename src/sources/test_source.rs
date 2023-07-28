use std::error::Error;

use async_trait::async_trait;
use reqwest::Client;

use crate::{
    scraping::{self, requests::Test},
    scraping_traits::{self, BaseTraits, Scraper},
};

impl scraping_traits::Source for scraping::requests::Test {
    fn get_source_name(&self) -> String {
        "test".to_string()
    }
}

impl BaseTraits for Test {}

#[async_trait]
impl scraping_traits::Scraper for Test {
    fn get_unique_id(&self) -> String {
        format!("Test-payload")
    }

    async fn scrape(
        &self,
        client: &Client,
    ) -> Result<scraping::results::ScrapingResult, Box<dyn Error + Send>> {
        todo!()
    }
}
