pub mod client;
pub mod layout;

// Re-export commonly used types
pub use client::{AiClient, AiClientError};
pub use layout::{AiLayoutError, AiLayoutResponse, AiPartitionLayout, WindowPlacement};
