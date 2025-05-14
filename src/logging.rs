use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{fmt, EnvFilter};


pub enum VerboseLevel {
    Verbose,
    Debug,
    Default,
}

pub fn init_logging(verbose: &VerboseLevel) {
    let filter = match verbose {
        VerboseLevel::Verbose => {
            EnvFilter::new("info,Rustique=info,ureq=info,tokio=info,tokio_runtime=info")
        }
        VerboseLevel::Debug => {
            EnvFilter::new("info,Rustique=debug,ureq=info,tokio=info,tokio_runtime=info")
        }
        VerboseLevel::Default => {
            EnvFilter::new("info,Rustique=warn,ureq=warn,tokio=debug,tokio_runtime=debug")
        }
    };

    tracing_subscriber::registry()
        .with(fmt::layer()
            .with_thread_ids(true)
            .with_thread_names(true)
            .with_target(false))
        .with(filter)
        .init();
}