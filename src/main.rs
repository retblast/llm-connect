use std::{thread::sleep, time::Duration};

use crate::connection::{
    koboldcpp_start, koboldcpp_tts_send_prompt, llama_send_prompt, process_killer,
};
use clap::Parser;

mod connection;

#[derive(Parser)]
#[command(name = "llm-connect")]
#[command(
    version,
    about = "Connect to a local LLM",
    long_about = "Mostly for testing ATM."
)]

// TODO: Put stuff that only belongs to a certain category under a sub struct?
// Dunno if that's possible
struct Cli {
    /// Mode
    #[arg(short, long)]
    mode: Option<String>,
    /// Host address
    #[arg(short = 'A', long)]
    host: Option<String>,
    /// Port
    #[arg(short, long)]
    port: Option<u32>,
    /// Directory where models are
    #[arg(short = 'M', long)]
    model_dir: Option<String>,
    /// Directory where the voice references are
    #[arg(short, long)]
    voice_refs_dir: Option<String>,
    /// Line to voice
    #[arg(short, long)]
    text: Option<String>,
    /// Voice reference file
    #[arg(short = 'r', long)]
    voice_reference_file: Option<String>,
    /// Output filename
    #[arg(short, long)]
    output_filename: Option<String>,
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    let mode = match cli.mode {
        Some(string) => string,
        None => panic!("No mode specified."),
    };
    let host = match cli.host {
        Some(host) => host,
        None => {
            println!("No host specified, defaulting to localhost");
            "localhost".to_string()
        }
    };
    let port = match cli.port {
        Some(port) => port,
        None => {
            println!("No port specified, defaulting to 5001");
            5001
        }
    };

    let mut process_pid: u32 = 0;

    match mode.as_str() {
        "tts" => {
            let model_dir = match cli.model_dir {
                Some(string) => string,
                None => panic!("No model directory specified."),
            };
            let voice_refs_dir = match cli.voice_refs_dir {
                Some(string) => string,
                None => panic!("No voice references directory specified."),
            };
            let text = match cli.text {
                Some(text) => text,
                None => panic!("No text to voice specified."),
            };
            let voice_reference_file = match cli.voice_reference_file {
                Some(voice_reference_file) => voice_reference_file,
                None => {
                    println!("No voice reference file specified, output will use a random voice.");
                    "".to_string()
                }
            };
            let output_filename = match cli.output_filename {
                Some(output_filename) => output_filename,
                None => {
                    println!("Filename not set. Will be saved as 'voiced_file.mp3'");
                    "voiced_file.mp3".to_string()
                }
            };

            process_pid =
                match koboldcpp_start(&mode.to_string(), &host, &port, &model_dir, &voice_refs_dir)
                    .await
                {
                    Ok(pid) => pid,
                    Err(_) => panic!("Failed to start koboldcpp"),
                };
            let koboldtts_result = koboldcpp_tts_send_prompt(
                &"http://localhost:5001/v1/audio/speech".to_owned(),
                &output_filename.to_owned(),
                &"kcpp".to_owned(),
                &text.to_owned(),
                &voice_reference_file.to_owned(),
            )
            .await;
            match koboldtts_result {
                Ok(_) => (),
                Err(_) => (),
            }
        }
        "chat" => todo!("Add stuff about chatting"),
        "music" => todo!("Add stuff about music"),
        &_ => todo!("Deal with this"),
    }

    process_killer(&process_pid);
    println!("Thanks for using llm-connect!");
}
