// All about connections
use anyhow::{Context, Result};
use futures_util::StreamExt;
use std::io::Write;
use std::thread::{self, sleep};
use std::time::Duration;
use std::{fs::File, process::Command};
use sysinfo::{ProcessRefreshKind, RefreshKind, System};

#[derive(serde::Serialize, serde::Deserialize)]
pub struct Message {
    pub role: String,
    pub content: String,
}

#[derive(serde::Serialize)]
struct LlamaRequest {
    messages: Vec<Message>,
    temperature: f32,
    max_tokens: u32,
}

#[derive(serde::Deserialize)]
pub struct LlamaResponseChoices {
    pub message: Message,
}

#[derive(serde::Deserialize)]
pub struct LlamaResponse {
    pub choices: Vec<LlamaResponseChoices>,
}

#[derive(serde::Serialize)]
struct KoboldTTSRequest {
    model: String,
    input: String,
    voice: String,
}

fn build_llama_prompt(
    system_prompt: &String,
    user_prompt: &String,
    temperature: &f32,
    max_tokens: &u32,
) -> LlamaRequest {
    let system = Message {
        role: "system".to_string(),
        content: system_prompt.to_string(),
    };

    let user = Message {
        role: "user".to_string(),
        content: user_prompt.to_string(),
    };
    let request = LlamaRequest {
        messages: vec![system, user],
        temperature: temperature.clone(),
        max_tokens: max_tokens.clone(),
    };
    return request;
}

// Sends the prompt, and if all goes well
// it returns the response, which is a vector of
// "choices"
pub async fn llama_send_prompt(
    destination: &String,
    system_prompt: &String,
    user_prompt: &String,
    temperature: &f32,
    max_tokens: &u32,
) -> Result<LlamaResponse, reqwest::Error> {
    let client = reqwest::Client::new();
    let request = build_llama_prompt(system_prompt, user_prompt, temperature, max_tokens);
    let response: LlamaResponse = client
        .post(destination)
        .json(&request)
        .send()
        .await?
        .json()
        .await?;
    return Ok(response);
}

fn koboldcpp_configure_tts(
    model_dir: &String,
    voice_refs_dir: &String,
    original_command: Command,
) -> Command {
    let mut tts_command = Command::from(original_command);
    tts_command
        .arg("--ttsgpu")
        .arg("--ttsmodel")
        .arg(format!("{model_dir}/Qwen3-TTS-12Hz-1.7B-Base-q8_0.gguf"))
        .arg("--ttswavtokenizer")
        .arg(format!("{model_dir}/qwen3-tts-tokenizer-q8_0.gguf"))
        .arg("--ttsdir")
        .arg(format!("{voice_refs_dir}"));
    return tts_command;
}

pub async fn check_llm_alive_yet(host: &String, port: &u32, retries: &u8) -> bool {
    let client = reqwest::Client::new();
    let mut response = client.get(format!("http://{host}:{port}")).send().await;
    let mut response_code: u16 = 0;
    let mut our_retries = retries.clone();
    while our_retries > 0 {
        sleep(Duration::new(1, 0));
        our_retries -= 1;
        response = client.get(format!("http://{host}:{port}")).send().await;
        response_code = match response {
            Ok(response_result) => response_result.status().as_u16(),
            Err(_) => 0,
        };
    }
    match response_code {
        200 => true,
        _ => false,
    }
}
// Returns the PID of the running koboldcpp instance
pub async fn koboldcpp_start(
    mode: &String,
    host: &String,
    port: &u32,
    model_dir: &String,
    voice_refs_dir: &String,
) -> Result<u32, Box<dyn std::error::Error>> {
    let mut main_command = Command::new("koboldcpp");
    main_command
        .arg("--host")
        .arg(format!("{host}"))
        .arg("--port")
        .arg(format!("{port}"))
        .arg("--gpulayers")
        .arg("-1")
        .arg("--threads")
        .arg("16")
        .arg("--usevulkan");

    // Just to have it initialized
    let mut final_command = Command::new("ls");
    match mode.as_str() {
        "tts" => final_command = koboldcpp_configure_tts(model_dir, voice_refs_dir, main_command),
        &_ => println!("Whoops @ koboldcpp_start"),
    }
    // KoboldCPP puts initialization details here, and its last line includes where the http api lies
    let stdout_file = File::create("koboldcpp_stdout.txt")?;
    // And it puts here details about the generation operations
    let stderr_file = File::create("koboldcpp_stderr.txt")?;

    // TODO: Make this print only by a flag
    //println!("{:?}", final_command);
    //
    final_command.stdout(stdout_file);
    final_command.stderr(stderr_file);
    let koboldcpp_process = final_command.spawn()?;

    if !check_llm_alive_yet(host, port, &10_u8).await {
        process_killer(&koboldcpp_process.id());
        panic!("koboldcpp took too long to start!");
    };
    //TODO: Having the PID is good for cleanup, but we have to check that it is up already
    return Ok(koboldcpp_process.id());
}

fn koboldcpp_tts_build_prompt(model: &String, input: &String, voice: &String) -> KoboldTTSRequest {
    let request = KoboldTTSRequest {
        model: model.clone(),
        input: input.clone(),
        voice: voice.clone(),
    };
    return request;
}

pub async fn koboldcpp_tts_send_prompt(
    destination: &String,
    output_filename: &String,
    model: &String,
    input: &String,
    voice: &String,
) -> Result<File> {
    let client = reqwest::Client::new();
    let request = koboldcpp_tts_build_prompt(model, input, voice);
    let response = client
        .post(destination)
        .json(&request)
        .send()
        .await
        .context("Failed to send request to KoboldCPP")?;
    //println!("{:?}", response);
    let mut output_file = File::create(output_filename).context("Failed to create output file")?;
    let mut stream = response.bytes_stream();
    while let Some(chunk) = stream.next().await {
        output_file
            .write_all(&chunk?)
            .expect("Failed writing the chunks");
    }
    Ok(output_file)
}

// Just kills a process by its pid
pub fn process_killer(pid_to_kill: &u32) {
    // Only get processes, without tasks
    let sys = System::new_with_specifics(
        RefreshKind::nothing().with_processes(ProcessRefreshKind::everything().without_tasks()),
    );
    // Refresh
    let koboldcpp_process = match sys.process(sysinfo::Pid::from_u32(*pid_to_kill)) {
        Some(process) => process,
        None => panic!("Something went wrong: Koboldcpp PID is wrong."),
    };
    match koboldcpp_process.kill_and_wait() {
        Ok(result) => match result {
            Some(exit_status) => println!("Koboldcpp exited with status: {}", exit_status),
            None => panic!("Something happened when trying to wait and kill koboldcpp"),
        },
        Err(error) => panic!(
            "Something went wrong when trying to kill and wait koboldcpp: {}",
            error
        ),
    };
}
// TODO: Revisit
//fn build_prompt(
//    model: &String,
//    system_prompt: &String,
//    user_prompt: &String,
//    temperature: &f32,
//    max_tokens: &u32,
//) {
//    match model {
//        "llama" => build_llama_prompt(system_prompt, user_prompt, temperature, max_tokens),
//    }
//}
//
