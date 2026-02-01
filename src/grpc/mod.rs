pub mod client;
pub mod server;

pub use server::{run_grpc_server, LogProcessorService};

// Re-export proto module for client use
pub mod log_processor {
    tonic::include_proto!("log_processor");
}

pub use log_processor::FilterOptions;
