use reqwest::Client;
use tokio::sync::mpsc::{Receiver, Sender};

use crate::{
    pubsub::{get_access_token, OutboundPubSubPayload},
    scraping::results::ScrapingResult,
    BoxedErr,
};

pub(crate) async fn spawn_postal_service(
    mut postal_rx: Receiver<ScrapingResult>,
    errors_tx: Sender<BoxedErr>,
) {
    // CONSTANTS
    let PUBLISH_TOPIC_ENDPOINT: String =
        std::env::var("PUBLISH_TOPIC").expect("Missing PUBLISH_TOPIC env variable.");

    // Logging
    println!("Starting up postal service.");

    // Creating a local reqwest client
    let client = reqwest::Client::new();

    // Creating read loop
    while let Some(scraping_result) = postal_rx.recv().await {
        // Serializing the payload
        match OutboundPubSubPayload::from(scraping_result).serialize_payload() {
            Ok(serialized_payload) => {
                // Publishing the payload
                let mut count = 0;
                loop {
                    count += 1; // Incrementing the count
                    match publish_payload(
                        &client,
                        &PUBLISH_TOPIC_ENDPOINT,
                        serialized_payload.clone(),
                    )
                    .await
                    {
                        Ok(response) if response.status().is_success() => {
                            match deserialize_response(response).await {
                                Ok(deserialized_response) => {
                                    // Simply logging the response for now
                                    println!("Successfully published payload. See response below:");
                                    println!("{:#?}", deserialized_response);
                                }
                                Err(e) => {
                                    // Sending the error to the error channel
                                    errors_tx.send(Box::new(e)).await.expect(
                                        "Unexpected error when sending to the error channel.",
                                    );
                                }
                            }
                            break;
                        }
                        unsuccessful => {
                            // Getting the error
                            let err = match unsuccessful {
                                Ok(response) => {
                                    response.error_for_status().unwrap_err()
                                    // Should always return an error.
                                }
                                Err(e) => e,
                            };

                            // Sending the error to the error channel
                            errors_tx
                                .send(Box::new(err))
                                .await
                                .expect("Unexpected error when sending to the error channel.");
                            if count >= 5 {
                                break;
                            }
                        }
                    }
                }
            }
            Err(e) => {
                errors_tx
                    .send(Box::new(e))
                    .await
                    .expect("Unexpected error when sending to the error channel.");
            }
        }
    }

    unreachable!("Postal service has stopped.")
}

async fn publish_payload(
    client: &Client,
    topic: &str,
    serialized_payload: String,
) -> Result<reqwest::Response, reqwest::Error> {
    // Getting the access token
    let auth_details = get_access_token(client)
        .await
        .expect("Failed to get access token.");

    client
        .post(topic)
        .header(
            "Authorization",
            format!("Bearer {}", auth_details.access_token),
        )
        .body(serialized_payload)
        .send()
        .await?
        .error_for_status()
}

async fn deserialize_response(response: reqwest::Response) -> Result<String, reqwest::Error> {
    response.text().await
}
