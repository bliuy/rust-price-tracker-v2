pub mod scraping;
fn main() {
    println!("Hello, world!");
}

/// This module will contain all traits that should be implemented onto the individual scraping objects defined in the protobuf payload.
pub mod scraping_traits {
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
        fn find_css_node(document: &Html, selector_str: &str) -> Result<Node, CssError> {
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

        fn get_current_utc_time() -> u64 {
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
    pub trait Scraper: BaseTraits + Source {
        async fn scrape(
            &self,
            client: &Client,
        ) -> Result<scraping::results::ScrapingResult, Box<dyn std::error::Error + Send>>;
    }
}

pub mod sources {
    pub mod test_source {
        use crate::{scraping, scraping_traits};

        impl scraping_traits::Source for scraping::requests::Test {
            fn get_source_name(&self) -> String {
                "test".to_string()
            }
        }
    }

    pub mod amzn_source {
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
                let utc_timestamp = <Amzn as BaseTraits>::get_current_utc_time();

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
                    <Amzn as BaseTraits>::find_css_node(&document, PRODUCT_TITLE_SELECTOR_STR)?
                {
                    Ok(txt.trim().to_string())
                } else {
                    Err(CssError::new("Invalid Node found.").into())
                }
            }

            fn get_product_price(&self, document: &Html) -> Result<f32, BoxedErr> {
                const PRODUCT_PRICE_SELECTOR_STR: &str = ".a-offscreen"; // Contains a text string of the price (e.g. $75.99)

                if let scraper::Node::Text(txt) =
                    <Amzn as BaseTraits>::find_css_node(&document, PRODUCT_PRICE_SELECTOR_STR)?
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
            async fn scrape(
                &self,
                client: &Client,
            ) -> Result<scraping::results::ScrapingResult, Box<dyn Error + Send>> {
                // Constructing the request
                let request = self
                    .construct_request(client)
                    .map_err(|e| Box::new(e) as BoxedErr)?; // 'As' used to coerce concrete type into trait object.

                // Performing the request
                let raw_html_string = <Amzn as BaseTraits>::request(client, request, None, None)
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
    }
}

pub(crate) mod errors {
    use std::{error::Error, fmt::Display};

    use scraper::error::SelectorErrorKind;

    #[derive(Debug)]
    pub struct CssError {
        error_msg: String,
    }

    impl Display for CssError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{}", self.error_msg)
        }
    }

    impl Error for CssError {}

    impl CssError {
        pub(crate) fn new(error_msg: &str) -> Self {
            CssError {
                error_msg: error_msg.to_string(),
            }
        }
    }

    impl From<SelectorErrorKind<'_>> for CssError {
        fn from(value: SelectorErrorKind) -> Self {
            let error_msg = value.to_string();
            CssError { error_msg }
        }
    }
    
    impl From<CssError> for Box<dyn Error + Send> {
        fn from(value: CssError) -> Self {
            Box::new(value) as Box<dyn Error + Send>
        }
    }
}
