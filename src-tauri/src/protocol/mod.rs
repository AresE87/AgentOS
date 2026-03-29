pub mod spec;
pub mod server;
pub mod client;

pub use spec::{AAPMessage, AAPMessageType};
pub use server::AAPServer;
pub use client::AAPClient;
