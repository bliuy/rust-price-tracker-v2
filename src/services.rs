use std::error::Error;

use actix_web::{
    web::{Data, Json},
    HttpResponse, Responder,
};
use reqwest::Client;
use tokio::sync::mpsc::Sender;

use crate::{
    pubsub::PubSubMessage, scraping::results::ScrapingResult, scraping_traits::Scraper, BoxedErr,
};

pub(crate) async fn hello_world() -> impl Responder {
    HttpResponse::Ok().body("Hello World!")
}

pub(crate) async fn scraping_request_handler(
    json_payload: Json<PubSubMessage>,
    request_client: Data<Client>,
    result_channel: Data<Sender<ScrapingResult>>,
    errors_channel: Data<Sender<BoxedErr>>,
) -> impl Responder {
    // Decoding the inner payload
    let payload = json_payload.into_inner();

    // Unpacking the scraping requests
    let scraping_requests = match payload.get_scraping_requests() {
        Ok(i) => i,
        Err(e) => return HttpResponse::BadRequest().body::<String>(e.to_string()),
    };
    let request_count = scraping_requests.len();

    // Spawning a separate async thread to execute the scraping requests
    tokio::task::spawn(async move {
        scraping_request(
            scraping_requests,
            request_client,
            result_channel,
            errors_channel,
        )
        .await
    });

    let response_msg = format!(
        "Scraping requests acknowleged.\nNumber of scraping requests recieved: {}",
        request_count
    );
    HttpResponse::Ok().body(response_msg)
}

pub(crate) async fn scraping_request(
    scraping_requests: Vec<Box<dyn Scraper + Send>>,
    request_client: Data<Client>,
    result_channel: Data<Sender<ScrapingResult>>,
    failed_channel: Data<Sender<BoxedErr>>,
) -> () {
    println!("Processing scraping request.");

    let mut tasks = tokio::task::JoinSet::new();
    println!("Number of scraping requests: {}", scraping_requests.len());
    for req in scraping_requests.into_iter() {
        let client = request_client.clone();

        // Spawning a separate task
        tasks.spawn(async move {
            let req = req.scrape(&client);
            req.await
        });
    }

    while let Some(thread_res) = tasks.join_next().await {
        match thread_res {
            Ok(res) => match res {
                Ok(i) => {
                    match result_channel.send(i).await {
                        Ok(_) => {}
                        Err(e) => {
                            println!("Error occured when sending the ScrapingResult across the mpsc channel. See error:");
                            println!("{}", e);
                        }
                    };
                }
                Err(e) => match failed_channel.send(e).await {
                    Ok(_) => {}
                    Err(internal_err) => {
                        println!("Error occured when sending the Error raised during the scraping process across the mpsc channel. See error:");
                        println!("{}", internal_err);
                    }
                },
            },
            Err(e) => {
                println!("JoinError encountered. See error below:");
                println!("{}", e);
            }
        }
    }

    println!("Scraping request processed.");
}
