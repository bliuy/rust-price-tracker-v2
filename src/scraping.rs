pub mod requests {
    include!(concat!(env!("OUT_DIR"), "/scraping.requests.rs"));
}
pub mod results {
    include!(concat!(env!("OUT_DIR"), "/scraping.results.rs"));
}
