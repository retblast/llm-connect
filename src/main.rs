use crate::connection::{koboldcpp_start, llama_send_prompt};
use clap::Parser;

mod connection;

#[derive(Parser)]
#[command(name = "llm-connect")]
#[command(
    version,
    about = "Connect to a local LLM",
    long_about = "Mostly for testing ATM."
)]
struct Cli {
    /// Directory where models are
    #[arg(short, long)]
    model_dir: Option<String>,
    /// Directory where the voice references are
    #[arg(short, long)]
    voice_refs_dir: Option<String>,
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    let model_dir = match cli.model_dir {
        Some(string) => string,
        None => panic!("No model dir specified."),
    };
    let voice_refs_dir = match cli.voice_refs_dir {
        Some(string) => string,
        None => panic!("No model dir specified."),
    };
    koboldcpp_start(&"tts".to_string(), &model_dir, &voice_refs_dir);

}
