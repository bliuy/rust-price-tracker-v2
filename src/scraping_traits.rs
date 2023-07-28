use std::{
    ops::Div,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use async_trait::async_trait;
use reqwest::{Client, Request, Response};
use scraper::{Html, Node};

use crate::{errors::CssError, scraping};

/// Common scraping functions that should be implemented across all structs.
/// All methods implemented here should have a default implementation.
#[async_trait]
pub trait BaseTraits {
    fn find_css_node(&self, document: &Html, selector_str: &str) -> Result<Node, CssError> {
        // Creating the selector
        let selector = scraper::Selector::parse(selector_str)?;

        // Finding the node within the document
        let node = document
            .select(&selector)
            .next()
            .ok_or_else(|| CssError::new("Failed to find Css Node."))?
            .first_child()
            .ok_or_else(|| CssError::new("Failed to find Css element."))?
            .value();

        Ok(node.to_owned())
    }

    async fn request(
        &self,
        client: &Client,
        request: Request,
        max_retries: Option<u32>,
        exponential_backoff_algo: Option<fn(u32) -> u32>,
    ) -> Result<Response, reqwest::Error> {
        // Configuring max retries
        let _max_retries = match max_retries {
            Some(i) if i > 10 => 10,
            Some(i) => i,
            None => 5,
        };

        // Configuring exponential_backoff_algo
        let _exponential_backoff_algo = match exponential_backoff_algo {
            Some(i) => i,
            None => |count| (2_u32.pow(count) - 1).div(2),
        };

        // Performing the request
        let mut i = 0;
        while i < _max_retries {
            let request = request.try_clone().expect("Unreachable!"); // All request types should be cloneable in this case.
            let result = client.execute(request).await?;
            i += 1; // Incrementing the counter after making the request.
            match result.error_for_status() {
                Ok(i) => {
                    return Ok(i);
                }
                Err(_) if i < _max_retries => {
                    let sleep_seconds = _exponential_backoff_algo(i);
                    let sleep_duration = Duration::from_secs(sleep_seconds.into());
                    tokio::time::sleep(sleep_duration).await;
                }
                Err(e) => return Err(e),
            }
        }

        unreachable!()
    }

    fn get_current_utc_time(&self) -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Unable to retrieve system time.")
            .as_secs()
    }
}

pub trait Source {
    fn get_source_name(&self) -> String;
}

#[async_trait]
pub trait Scraper: BaseTraits + Source + Send {
    /// This method should return a unique identifier for the targeted scraping product.
    fn get_unique_id(&self) -> String;

    async fn scrape(
        &self,
        client: &Client,
    ) -> Result<scraping::results::ScrapingResult, Box<dyn std::error::Error + Send>>;
}
