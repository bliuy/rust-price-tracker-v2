use std::{collections::HashMap, error::Error};

use base64::{DecodeError, Engine};
use prost::EncodeError;
use reqwest::Client;

use crate::{
    scraping::{
        json_results::ScrapingResultJson, requests::ScrapingRequests, results::ScrapingResult,
    },
    scraping_traits::Scraper,
};

type BoxedErr = Box<dyn Error + Send>;

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub(crate) struct PubSubMessage {
    pub(crate) message: PubSubMessageMessage,
    pub(crate) subscription: String,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub(crate) struct AuthDetails {
    pub(crate) access_token: String,
    pub(crate) expires_in: i32,
    pub(crate) token_type: String,
}
pub(crate) async fn get_access_token(client: &Client) -> Result<AuthDetails, BoxedErr> {
    const URL_ENDPOINT: &str = "http://metadata.google.internal/computeMetadata/v1/instance/service-accounts/default/token";

    // Constructing the request call to the metadata server
    let raw_response: String = client
        .get(URL_ENDPOINT)
        .header("Metadata-Flavor", "Google")
        .send()
        .await
        .map_err(|e| Box::new(e) as BoxedErr)?
        .text()
        .await
        .map_err(|e| Box::new(e) as BoxedErr)?;

    let response: AuthDetails =
        serde_json::from_str(&raw_response).map_err(|e| Box::new(e) as BoxedErr)?;

    Ok(response)
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
#[allow(non_snake_case)]
pub(crate) struct PubSubMessageMessage {
    pub(crate) data: String, // Base64 encoded payload
    pub(crate) attributes: Option<HashMap<String, String>>,
    pub(crate) messageId: Option<String>,
    pub(crate) message_id: Option<String>,
    pub(crate) publishTime: Option<String>,
    pub(crate) publish_time: Option<String>,
}

/// This struct will be the request payload when sent to the PubSub service.
#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub(crate) struct OutboundPubSubPayload {
    messages: Vec<PubSubMessageMessage>,
}

impl FromIterator<ScrapingResult> for OutboundPubSubPayload {
    fn from_iter<T: IntoIterator<Item = ScrapingResult>>(iter: T) -> Self {
        let messages = iter
            .into_iter()
            .map(|i| {i.encode_to_pubsub().expect("Unexpected EncodeError raised when encoding the scraping results into a pubsub message.")})
            .collect::<Vec<_>>();

        OutboundPubSubPayload { messages }
    }
}

impl OutboundPubSubPayload {
    pub fn serialize_payload(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }
}

impl PubSubMessage {
    fn decode_base64(self) -> Result<Vec<u8>, DecodeError> {
        let raw_payload = self.message.data; // Payload will come in a raw text string.
        base64::engine::general_purpose::STANDARD.decode(raw_payload) // Performing base64 decoding to decode the raw text string into a bytes sequence.
    }

    fn decode_scraping_requests(self) -> Result<ScrapingRequests, Box<dyn Error + Send>> {
        let payload_bytes = self.decode_base64().map_err(|e| Box::new(e) as BoxedErr)?;

        let scraping_requests: ScrapingRequests =
            prost::Message::decode(&*payload_bytes).map_err(|e| Box::new(e) as BoxedErr)?; // Decoding the bytes sequence into the correct object.

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

impl ScrapingResult {
    fn encode_to_pubsub(self) -> Result<PubSubMessageMessage, EncodeError> {
        let json_result = ScrapingResultJson::from(self);
        let serialized_payload = serde_json::to_string(&json_result)
            .expect("Unexpected error when serializing the ScrapingResult into a JSON string.");
        let base64_encoded = base64::engine::general_purpose::STANDARD.encode(serialized_payload);
        Ok(PubSubMessageMessage {
            data: base64_encoded,
            attributes: None,
            messageId: None,
            message_id: None,
            publishTime: None,
            publish_time: None,
        })
    }
}

impl From<ScrapingResult> for OutboundPubSubPayload {
    fn from(value: ScrapingResult) -> Self {
        vec![value].into_iter().collect()
    }
}
