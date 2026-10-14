// All about connections
use anyhow::Result;
use futures_util::StreamExt;
use reqwest::Client;
use std::fmt::Display;
use std::fmt::Formatter;
use std::sync::LazyLock;
use std::time::Duration;
use tokio::io::AsyncWriteExt;
use tokio::time::sleep;

use crate::config;

static HTTP_CLIENT: LazyLock<Client> = LazyLock::new(|| {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .connect_timeout(Duration::from_secs(5))
        .build()
        .expect("Failed to initialize the HTTTP client.")
});

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

pub async fn check_llm_alive_yet(address: &str, max_retries: u8) -> Result<(), Error> {
    let mut alive = false;
    let mut interval = 1;
    let mut retries = max_retries;
    while !alive && retries != 0 {
        println!("Checking if the openai api endpoint is alive");
        println!("Retry: {}", retries);
        sleep(Duration::new(interval, 0)).await;
        let response = HTTP_CLIENT.get(format!("{address}")).send().await;
        let response_code = match response {
            Ok(response_result) => response_result.status().as_u16(),
            Err(_) => 0,
        };
        alive = match response_code {
            200 => true,
            _ => false,
        };
        retries -= 1;
        interval *= 2;
    }
    match alive {
        true => return Ok(()),
        false => return Err(Error::NotReady),
    }
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
pub enum Error {
    Request(reqwest::Error),
    Decode(reqwest::Error),
    Body(reqwest::Error),
    IoError(std::io::Error),
    NotReady,
    UnexpectedResponse {
        code: u16,
        reason: String,
        body: String,
    },
    EmptyResponse,
    ConfigError(config::Error),
}

impl From<reqwest::Error> for Error {
    fn from(error: reqwest::Error) -> Self {
        if error.is_decode() {
            return Self::Decode(error);
        }
        if error.is_body() {
            return Self::Body(error);
        }

        Self::Request(error)
    }
}

impl From<config::Error> for Error {
    fn from(error: config::Error) -> Self {
        Self::ConfigError(error)
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Request(error) | Error::Body(error) | Error::Decode(error) => {
                std::error::Error::source(error)
            }
            Error::IoError(error) => std::error::Error::source(error),
            Error::ConfigError(error) => std::error::Error::source(error),
            Error::UnexpectedResponse { .. } | Error::EmptyResponse | Error::NotReady => None,
        }
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Request(error) => match error.url() {
                Some(url) => write!(f, "Failed to send request to {url} due to {error}"),
                None => write!(f, "Request never reached server due to {error}"),
            },
            Self::Decode(error) => write!(f, "Can't parse server response due to {error}"),
            Self::Body(error) => write!(
                f,
                "Connection lost when reading the response body due to {error}"
            ),
            Self::IoError(error) => write!(f, "File operation failed due to {error}"),
            Self::NotReady => write!(f, "LLM wasn't ready on time."),
            Self::UnexpectedResponse { code, reason, body } => {
                // It's recommended to truncate the body here
                let truncated_body: String = body.chars().take(500).collect();
                write!(
                    f,
                    "Unexpected response.\nCode: {code}\nReason: {reason}\nBody: {truncated_body}",
                )
            }
            Self::EmptyResponse => {
                write!(f, "got an empty response")
            }
            Self::ConfigError(error) => write!(f, "{error}"),
        }
    }
}

impl From<std::io::Error> for Error {
    fn from(error: std::io::Error) -> Self {
        Self::IoError(error)
    }
}

// check if we are getting something usable
async fn ensure_success(response: reqwest::Response) -> Result<reqwest::Response, Error> {
    let status = response.status();
    if !status.is_success() {
        let body = response.text().await?;
        return Err(Error::UnexpectedResponse {
            code: status.as_u16(),
            reason: status
                .canonical_reason()
                .unwrap_or("unknown reason")
                .to_owned(),
            body,
        });
    }
    Ok(response)
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
) -> Result<OpenAIChatResponse, Error> {
    let request = openai_chat_build_prompt(system_prompt, user_prompt, temperature, max_tokens);
    check_llm_alive_yet(address, max_retries).await?;
    let response = ensure_success(
        HTTP_CLIENT
            .post(address.to_owned() + "/v1/chat/completions")
            .json(&request)
            .send()
            .await?,
    )
    .await?;

    let response: OpenAIChatResponse = response.json().await?;
    if response.choices.is_empty() {
        return Err(Error::EmptyResponse);
    }
    Ok(response)
}

pub async fn openai_tts_send_prompt(
    address: &str,
    output_filename: &str,
    model: &str,
    input: &str,
    voice: &str,
    max_retries: u8,
) -> Result<(), Error> {
    let request = openai_tts_build_prompt(model, input, voice);
    check_llm_alive_yet(address, max_retries).await?;
    let response = ensure_success(
        HTTP_CLIENT
            .post(address.to_owned() + "/v1/audio/speech")
            .json(&request)
            .send()
            .await?,
    )
    .await?;

    let mut output_file = tokio::fs::File::create(output_filename).await?;
    let mut stream = response.bytes_stream();
    let mut file_outcome = Ok(());
    while let Some(chunk) = stream.next().await {
        let chunk = match chunk {
            Ok(chunk) => chunk,
            Err(why) => {
                file_outcome = Err(why.into());
                break;
            }
        };

        if let Err(why) = output_file.write_all(&chunk).await {
            file_outcome = Err(why.into());
            break;
        }
    }

    match file_outcome {
        Ok(()) => {
            if let Err(flush_why) = output_file.flush().await {
                if let Err(io_remove_why) = tokio::fs::remove_file(output_filename).await {
                    eprintln!(
                        "failed to remove the partial file {output_filename} due to {io_remove_why}"
                    )
                }
                file_outcome = Err(flush_why.into());
            }
        }
        Err(why) => {
            if let Err(io_why) = tokio::fs::remove_file(output_filename).await {
                eprintln!("failed to remove the partial file {output_filename} due to {io_why}")
            }
            return Err(why);
        }
    }

    file_outcome
}
