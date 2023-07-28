use actix_web::{
    guard::{Get, Post},
    web::{self, resource, route},
    App, HttpServer,
};
use reqwest::ClientBuilder;

use crate::services::{hello_world, scraping_request_handler};

pub(crate) mod errors;
pub(crate) mod pubsub;
pub mod scraping;
pub mod scraping_traits;
pub mod sources;

#[actix_web::main]
async fn main() {
    // Defining consts
    const USER_AGENT: &str = "Mozilla/5.0 (Windows NT 6.1; WOW64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/47.0.2526.111 Safari/537.36";

    // Logging service start
    println!("Scraper service starting.");

    // Starting up the HTTPServer
    HttpServer::new(|| {
        // Constructing reqwest client service
        let req_client = ClientBuilder::new()
            .user_agent(USER_AGENT)
            .build()
            .expect("Failed to build reqwest client.");

        // Constructing the App instance
        App::new()
            .app_data(web::Data::new(req_client))
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

    unreachable!()
}

pub(crate) mod services {
    use std::error::Error;

    use actix_web::{
        web::{Data, Json},
        HttpResponse, Responder,
    };
    use reqwest::Client;
    use tokio::sync::mpsc::Sender;

    use crate::{
        pubsub::PubSubMessage, scraping::results::ScrapingResult, scraping_traits::Scraper,
    };

    pub(crate) async fn hello_world() -> impl Responder {
        HttpResponse::Ok().body("Hello World!")
    }

    pub(crate) async fn scraping_request_handler(
        json_payload: Json<PubSubMessage>,
        request_client: Data<Client>,
        result_channel: Data<Sender<ScrapingResult>>,
        failed_channel: Data<Sender<Box<dyn Error + Send>>>,
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
                failed_channel,
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
        failed_channel: Data<Sender<Box<dyn Error + Send>>>,
    ) -> () {
        let mut tasks = tokio::task::JoinSet::new();
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
                            println!("Error occured when sending the ScrapingResult across the mpsc channel. See error:");
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

        todo!()
    }
}

pub(crate) mod postal {
    use tokio::sync::mpsc;

    // async fn spawn_postal_service() -> mpsc::Sender<()> {
    //     // Logging the spawn request
    //     println!("Starting up postal service.");

    //     // Creating the channels
    //     let (tx, rx) = mpsc::channel(1024);

    //     // Spawning a separate thread that will run indefinitely

    // }
}
