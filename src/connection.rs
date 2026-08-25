// All about connections
use anyhow::{Context, Result};
use futures_util::StreamExt;
use std::fmt::Display;
use std::fmt::Formatter;
use std::io::Write;
use std::time::Duration;
use std::{fs::File, process::Command};
use sysinfo::{ProcessRefreshKind, RefreshKind, System};
use tokio::time::sleep;

use crate::config::KoboldConfig;

#[derive(serde::Serialize, serde::Deserialize)]
pub struct Message {
    pub role: String,
    pub content: String,
}

#[derive(serde::Serialize)]
struct OpenAIVoiceRequest {
    model: String,
    input: String,
    voice: String,
}

#[derive(serde::Serialize)]
struct OpenAIChatRequest {
    messages: Vec<Message>,
    temperature: f32,
    max_tokens: u32,
}

#[derive(serde::Deserialize)]
pub struct OpenAIChatResponseChoices {
    pub message: Message,
}

#[derive(serde::Deserialize)]
pub struct OpenAIChatResponse {
    pub choices: Vec<OpenAIChatResponseChoices>,
}

pub async fn check_llm_alive_yet(address: &str, max_retries: u8) -> bool {
    let client = reqwest::Client::new();
    let mut alive = false;
    let mut retries = max_retries;
    while !alive && retries != 0 {
        println!("Checking if the openai api endpoint is alive");
        println!("Retry: {}", retries);
        sleep(Duration::new(1, 0)).await;
        let response = client.get(format!("{address}")).send().await;
        let response_code = match response {
            Ok(response_result) => response_result.status().as_u16(),
            Err(_) => 0,
        };
        alive = match response_code {
            200 => true,
            _ => false,
        };
        retries -= 1;
    }
    return alive;
}

async fn koboldcpp_spawn(command: &mut tokio::process::Command) {
    loop {
        let mut koboldcpp_process = match command.spawn() {
            Ok(child) => child,
            Err(why) => panic!("Failed to spawn koboldcpp process, because of: {}", why),
        };
        let koboldcpp_status = koboldcpp_process.wait().await;

        match koboldcpp_status {
            Ok(_) => println!("Koboldcpp exited successfully."),
            Err(why) => println!("Kobold did not exit cleanly: {}", why),
        }
    }
}
// Starts koboldcpp
// TODO: make agnostic
pub async fn koboldcpp_start(kobold_config: &KoboldConfig) {
    // KoboldCPP puts initialization details here, and its last line includes where the http api lies
    let stdout_file = match File::create("koboldcpp_stdout.txt") {
        Ok(file) => file,
        Err(why) => panic!("Failed to create stdout file, because of {}", why),
    };
    // And it puts here details about the generation operations
    let stderr_file = match File::create("koboldcpp_stderr.txt") {
        Ok(file) => file,
        Err(why) => panic!("Failed to create stderr file, because of {}", why),
    };
    // Build the command
    let mut final_command = kobold_config.build_command();
    // TODO: Make this print only by a flag
    //println!("{:?}", final_command);
    //
    final_command.stdout(stdout_file);
    final_command.stderr(stderr_file);

    tokio::spawn(async move { koboldcpp_spawn(&mut final_command).await });
}

fn openai_tts_build_prompt(model: &str, input: &str, voice: &str) -> OpenAIVoiceRequest {
    let request = OpenAIVoiceRequest {
        model: model.to_string(),
        input: input.to_string(),
        voice: voice.to_string(),
    };
    return request;
}

fn openai_chat_build_prompt(
    system_prompt: &str,
    user_prompt: &str,
    temperature: &f32,
    max_tokens: &u32,
) -> OpenAIChatRequest {
    let system = Message {
        role: "system".to_string(),
        content: system_prompt.to_string(),
    };

    let user = Message {
        role: "user".to_string(),
        content: user_prompt.to_string(),
    };
    let request = OpenAIChatRequest {
        messages: vec![system, user],
        temperature: temperature.clone(),
        max_tokens: max_tokens.clone(),
    };
    return request;
}

#[derive(Debug)]
pub enum LlmConnectionError {
    Request(reqwest::Error),
    IoError(std::io::Error),
}

impl From<reqwest::Error> for LlmConnectionError {
    fn from(error: reqwest::Error) -> Self {
        Self::Request(error)
    }
}

impl Display for LlmConnectionError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Request(error) => write!(f, "Failed to send request: {error}"),
            Self::IoError(error) => write!(f, "Failed to save file: {error}"),
        }
    }
}

impl From<std::io::Error> for LlmConnectionError {
    fn from(error: std::io::Error) -> Self {
        Self::IoError(error)
    }
}

// Sends the prompt, and if all goes well
// it returns the response, which is a vector of
// "choices"
pub async fn openai_chat_send_prompt(
    address: &String,
    system_prompt: &String,
    user_prompt: &String,
    temperature: &f32,
    max_tokens: &u32,
    max_retries: u8,
) -> Result<OpenAIChatResponse, LlmConnectionError> {
    let client = reqwest::Client::new();
    let request = openai_chat_build_prompt(system_prompt, user_prompt, temperature, max_tokens);

    if !check_llm_alive_yet(address, max_retries).await {
        println!("Waiting for koboldcpp to be ready...");
    };
    let response: OpenAIChatResponse = client
        .post(address.to_owned() + "/v1/chat/completions")
        .json(&request)
        .send()
        .await?
        .json()
        .await?;
    return Ok(response);
}

//TODO: think about whether I should return File or () later
pub async fn openai_tts_send_prompt(
    address: &str,
    output_filename: &str,
    model: &str,
    input: &str,
    voice: &str,
    max_retries: u8,
) -> Result<File, LlmConnectionError> {
    let client = reqwest::Client::new();
    let request = openai_tts_build_prompt(model, input, voice);
    if !check_llm_alive_yet(address, max_retries).await {
        println!("Waiting for koboldcpp to be ready...");
    };
    let response = client
        .post(address.to_owned() + "/v1/audio/speech")
        .json(&request)
        .send()
        .await?;
    //println!("{:?}", response);
    let mut output_file = File::create(output_filename)?;
    let mut stream = response.bytes_stream();
    while let Some(chunk) = stream.next().await {
        output_file
            .write_all(&chunk?)
            .expect("Failed writing the chunks");
    }
    Ok(output_file)
}

// Just kills a process by its pid
pub fn process_killer(pid_to_kill: &u32, process_name: &String) {
    // Only get processes, without tasks
    let sys = System::new_with_specifics(
        RefreshKind::nothing().with_processes(ProcessRefreshKind::everything().without_tasks()),
    );
    // Refresh
    let process = match sys.process(sysinfo::Pid::from_u32(*pid_to_kill)) {
        Some(process) => process,
        None => panic!("Something went wrong: {} PID is wrong.", process_name),
    };
    match process.kill_and_wait() {
        Ok(result) => match result {
            Some(exit_status) => println!("{} exited with status: {}", process_name, exit_status),
            None => panic!(
                "Something happened when trying to wait and kill {}",
                process_name
            ),
        },
        Err(error) => panic!(
            "Something went wrong when trying to kill and wait for {}: {}",
            process_name, error
        ),
    };
}
