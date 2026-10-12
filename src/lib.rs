// pub mod chat;
pub mod config;
pub mod connection;
pub mod error;
pub mod process;

pub use connection::openai_tts_send_prompt;
pub use process::koboldcpp_start;
