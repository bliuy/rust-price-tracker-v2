use std::{error::Error, fmt::Display};

use scraper::error::SelectorErrorKind;
use tokio::sync::mpsc::Receiver;

type BoxedErr = Box<dyn Error + Send>;

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

pub(crate) async fn spawn_error_handler_service(mut errors_rx: Receiver<BoxedErr>) {
    // Logging the start of the error handler service
    println!("Starting error handler service...");

    while let Some(error) = errors_rx.recv().await {
        // TODO: Implement error handling
        // Logging the error to the console for now
        println!("Error: {}", error);
    }

    unreachable!("Error handler service has exited unexpectedly.")
}
