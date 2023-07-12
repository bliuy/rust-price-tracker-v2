fn main() {
    println!("Hello, world!");
}

pub mod Scraping {
    pub mod Requests {
        include!(
            concat!(
                env!(
                    "OUT_DIR"
                ),
                "/scraping.requests.rs"
            )
        );
    }
    pub mod Results {
        include!(
            concat!(
                env!(
                    "OUT_DIR"
                ),
                "/scraping.results.rs"
            )
        );
    }
}


pub mod Sources {

}