use std::{collections::HashMap, error::Error};

use base64::{DecodeError, Engine};

use crate::{scraping::requests::ScrapingRequests, scraping_traits::Scraper};

type BoxedErr = Box<dyn Error + Send>;

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub(crate) struct PubSubMessage {
    pub(crate) message: PubSubMessageMessage,
    pub(crate) subscription: String,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub(crate) struct PubSubMessageMessage {
    pub(crate) attributes: HashMap<String, String>,
    pub(crate) data: String, // Base64 encoded payload
    pub(crate) messageId: String,
    pub(crate) message_id: String,
    pub(crate) publishTime: String,
    pub(crate) publish_time: String,
}

impl PubSubMessage {
    fn decode_base64(self) -> Result<Vec<u8>, DecodeError> {
        let raw_payload = self.message.data;
        base64::engine::general_purpose::STANDARD.decode(raw_payload)
    }

    fn decode_scraping_requests(self) -> Result<ScrapingRequests, Box<dyn Error + Send>> {
        let payload_bytes = self.decode_base64().map_err(|e| Box::new(e) as BoxedErr)?;

        let scraping_requests: ScrapingRequests =
            prost::Message::decode(&*payload_bytes).map_err(|e| Box::new(e) as BoxedErr)?;

        Ok(scraping_requests)
    }

    pub(crate) fn get_scraping_requests(self) -> Result<Vec<Box<dyn Scraper + Send>>, BoxedErr> {
        let scraping_requests_wrapper = self.decode_scraping_requests()?;
        let result = scraping_requests_wrapper
            .requests
            .into_iter()
            .filter(|req| req.source.is_some())
            .map(|req| {
                let source = req.source.expect("Checked by filter step.");
                let scraper: Box<dyn Scraper + Send> = match source {
                    crate::scraping::requests::scraping_request::Source::Test(test) => {
                        Box::new(test)
                    }
                    crate::scraping::requests::scraping_request::Source::Amzn(amzn) => {
                        Box::new(amzn)
                    }
                };
                scraper
            })
            .collect::<Vec<_>>();
        Ok(result)
    }
}
