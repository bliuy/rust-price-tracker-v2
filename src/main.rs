use std::error::Error;

use actix_web::{
    guard::{Get, Post},
    web::{self, resource, route},
    App, HttpServer,
};
use reqwest::ClientBuilder;

use crate::{
    errors::spawn_error_handler_service,
    postal::spawn_postal_service,
    scraping::results::ScrapingResult,
    services::{hello_world, scraping_request_handler},
};

pub(crate) mod errors;
pub(crate) mod postal;
pub(crate) mod pubsub;
pub mod scraping;
pub mod scraping_traits;
pub(crate) mod services;
pub mod sources;

type BoxedErr = Box<dyn Error + Send>;

#[actix_web::main]
async fn main() {
    // Defining consts
    let USER_AGENT: String = std::env::var("USER_AGENT").expect("Missing USER_AGENT env variable.");

    // Logging service start
    println!("Scraper service starting.");

    // Creating the errors channel
    // This channel will handle all errors that are generated during the runtime of this serivce.
    let (errors_tx, errors_rx) = tokio::sync::mpsc::channel::<BoxedErr>(1024);

    // Creating channels for postal svc
    // The postal service will be responsible for the processing of the outbound messages
    let (postal_tx, postal_rx) = tokio::sync::mpsc::channel::<ScrapingResult>(1024);

    // Spawning the postal service
    let postal_svc_errors_tx = errors_tx.clone();
    tokio::task::spawn(async move { spawn_postal_service(postal_rx, postal_svc_errors_tx).await });

    // Spawning the error handler service
    tokio::task::spawn(async move {
        spawn_error_handler_service(errors_rx).await;
    });

    // Starting up the HTTPServer
    HttpServer::new(move || {
        // Constructing reqwest client service
        let req_client = ClientBuilder::new()
            .user_agent(&USER_AGENT)
            .build()
            .expect("Failed to build reqwest client.");

        // Constructing the App instance
        App::new()
            .app_data(web::Data::new(req_client))
            .app_data(web::Data::new(postal_tx.clone())) // Wrapped in a ARC
            .app_data(web::Data::new(errors_tx.clone())) // Wrapped in a ARC
            .service(resource("/hello-world").route(route().guard(Get()).to(hello_world)))
            .service(
                resource("/scraping-request")
                    .route(route().guard(Post()).to(scraping_request_handler)),
            )
    })
    .bind(("0.0.0.0", 8080))
    .expect("Failed to bind to requested port.")
    .run()
    .await
    .expect("Failed to start up HTTP server.");

    // unreachable!()
}
