use std::{collections::HashMap, error::Error};

use async_trait::async_trait;
use reqwest::{Client, Request};
use scraper::Html;

use crate::{
    errors::CssError,
    scraping::{self, requests::Amzn, results::ScrapingResult},
    scraping_traits::{self, BaseTraits, Source},
};

type BoxedErr = Box<dyn Error + Send>;

impl Amzn {
    fn construct_request(&self, client: &Client) -> reqwest::Result<Request> {
        // Constructing the target url
        let product_code = self.get_product_asin_code();
        let target_url = format!("https://www.amazon.sg/dp/{}", product_code);

        // Constructing the request
        let request = client.get(target_url).build()?;

        Ok(request)
    }

    fn get_product_asin_code(&self) -> String {
        self.product_code.to_owned()
    }

    fn get_product_information(&self, document: &Html) -> Result<ScrapingResult, BoxedErr> {
        // Getting the product_title
        let name = self.get_product_title(document)?;

        // Getting product price
        let price = self.get_product_price(document)?;

        // Getting current timestamp
        let utc_timestamp = self.get_current_utc_time();

        // Getting source
        let source = self.get_source_name();

        // Getting unique id
        let identifier = self.get_product_asin_code();

        // Constructing the ScrapingResult
        let result = ScrapingResult {
            source,
            utc_timestamp,
            name,
            identifier,
            price,
            attributes: HashMap::new(),
            metadata: HashMap::new(),
        };

        Ok(result)
    }

    fn get_product_title(&self, document: &Html) -> Result<String, BoxedErr> {
        const PRODUCT_TITLE_SELECTOR_STR: &str = "#productTitle";

        if let scraper::Node::Text(txt) =
            self.find_css_node(&document, PRODUCT_TITLE_SELECTOR_STR)?
        {
            Ok(txt.trim().to_string())
        } else {
            Err(CssError::new("Invalid Node found.").into())
        }
    }

    fn get_product_price(&self, document: &Html) -> Result<f32, BoxedErr> {
        const PRODUCT_PRICE_SELECTOR_STR: &str = ".a-offscreen"; // Contains a text string of the price (e.g. $75.99)

        if let scraper::Node::Text(txt) =
            self.find_css_node(&document, PRODUCT_PRICE_SELECTOR_STR)?
        {
            let price = txt
                .trim()
                .replace("$", "")
                .parse::<f32>()
                .map_err(|e| Box::new(e) as BoxedErr)?;
            Ok(price)
        } else {
            Err(CssError::new("Invalid Node found.").into())
        }
    }
}

impl scraping_traits::Source for Amzn {
    fn get_source_name(&self) -> String {
        "amzn".to_string()
    }
}

/// Using the default implementation for the BaseTraits
impl BaseTraits for Amzn {}

#[async_trait]
impl scraping_traits::Scraper for Amzn {
    fn get_unique_id(&self) -> String {
        let product_id = self.get_product_asin_code();
        let source = self.get_source_name();
        let unique_id = format!("{} - {}", source, product_id);
        unique_id
    }

    async fn scrape(
        &self,
        client: &Client,
    ) -> Result<scraping::results::ScrapingResult, Box<dyn Error + Send>> {
        // Constructing the request
        let request = self
            .construct_request(client)
            .map_err(|e| Box::new(e) as BoxedErr)?; // 'As' used to coerce concrete type into trait object.

        // Performing the request
        let raw_html_string = self
            .request(client, request, None, None)
            .await
            .map_err(|e| Box::new(e) as BoxedErr)?
            .text()
            .await
            .map_err(|e| Box::new(e) as BoxedErr)?;

        // Parsing the response into a HTML Document
        let document = Html::parse_document(&raw_html_string);

        // Extracting the relevant information from the HTML Document
        self.get_product_information(&document)
    }
}
