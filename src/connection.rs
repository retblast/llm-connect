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

// #[derive(serde::Serialize)]
// struct LlamaRequest {
//     messages: Vec<Message>,
//     temperature: f32,
//     max_tokens: u32,
// }

// #[derive(serde::Deserialize)]
// pub struct LlamaResponseChoices {
//     pub message: Message,
// }

// #[derive(serde::Deserialize)]
// pub struct LlamaResponse {
//     pub choices: Vec<LlamaResponseChoices>,
// }

// #[derive(serde::Serialize)]
// struct KoboldTTSRequest {
//     model: String,
//     input: String,
//     voice: String,
// }

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

struct KoboldTTSConfig {
    mode: String,
    host: String,
    port: u32,
    model_dir: String,
    voice_refs_dir: String,
}

// fn build_llama_prompt(
//     system_prompt: &String,
//     user_prompt: &String,
//     temperature: &f32,
//     max_tokens: &u32,
// ) -> OpenAIChatRequest {
//     let system = Message {
//         role: "system".to_string(),
//         content: system_prompt.to_string(),
//     };

//     let user = Message {
//         role: "user".to_string(),
//         content: user_prompt.to_string(),
//     };
//     let request = OpenAIChatRequest {
//         messages: vec![system, user],
//         temperature: temperature.clone(),
//         max_tokens: max_tokens.clone(),
//     };
//     return request;
// }

// Sends the prompt, and if all goes well
// it returns the response, which is a vector of
// "choices"
// pub async fn llama_send_prompt(
//     destination: &String,
//     system_prompt: &String,
//     user_prompt: &String,
//     temperature: &f32,
//     max_tokens: &u32,
// ) -> Result<OpenAIChatResponse, reqwest::Error> {
//     let client = reqwest::Client::new();
//     let request = build_llama_prompt(system_prompt, user_prompt, temperature, max_tokens);
//     let response: OpenAIChatResponse = client
//         .post(destination)
//         .json(&request)
//         .send()
//         .await?
//         .json()
//         .await?;
//     return Ok(response);
// }

// fn koboldcpp_configure_tts(
//     model_dir: &String,
//     voice_refs_dir: &String,
//     original_command: Command,
// ) -> Command {
//     let mut tts_command = Command::from(original_command);
//     tts_command

//     return tts_command;
// }

fn koboldcpp_configure_chat(model_dir: &String, original_command: Command) -> Command {
    let mut chat_command = Command::from(original_command);
    chat_command
        .arg("--model")
        .arg(format!("{model_dir}/pls_fill_me.gguf"));
    return chat_command;
}

pub async fn check_llm_alive_yet(address: &String) -> bool {
    let client = reqwest::Client::new();
    let mut alive = false;
    while !alive {
        println!("Checking if the openai api endpoint is alive");
        sleep(Duration::new(1, 0));
        let response = client.get(format!("{address}")).send().await;
        let response_code = match response {
            Ok(response_result) => response_result.status().as_u16(),
            Err(_) => 0,
        };
        alive = match response_code {
            200 => true,
            _ => false,
        };
    }
    return alive;
}

// Make it generic someday
impl KoboldTTSConfig {
    fn build_command(&self) -> tokio::process::Command {
        let host = &self.host;
        let port = &self.port;
        let model_dir = &self.model_dir;
        let voice_refs_dir = &self.voice_refs_dir;
        let mut main_command = tokio::process::Command::new("koboldcpp");
        main_command
            .arg("--host")
            .arg(format!("{host}"))
            .arg("--port")
            .arg(format!("{port}"))
            .arg("--gpulayers")
            .arg("-1")
            .arg("--threads")
            // TODO: Autodetect this
            // And optionally, let the user enter its value
            .arg("16")
            .arg("--usevulkan")
            .arg("--ttsgpu")
            .arg("--ttsmodel")
            .arg(format!("{model_dir}/Qwen3-TTS-12Hz-1.7B-Base-q8_0.gguf"))
            .arg("--ttswavtokenizer")
            .arg(format!("{model_dir}/qwen3-tts-tokenizer-q8_0.gguf"))
            .arg("--ttsdir")
            .arg(format!("{voice_refs_dir}"));
        main_command.kill_on_drop(true);
        main_command
    }
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
pub async fn koboldcpp_start(
    mode: &String,
    host: &String,
    port: &u32,
    model_dir: &String,
    voice_refs_dir: &String,
) {
    let kobold_config = KoboldTTSConfig {
        mode: mode.to_owned(),
        host: host.to_owned(),
        port: port.to_owned(),
        model_dir: model_dir.to_owned(),
        voice_refs_dir: voice_refs_dir.to_owned(),
    };
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

    // Just to have it initialized
    let mut final_command = tokio::process::Command::new("ls");
    match mode.as_str() {
        "tts" => {
            final_command = kobold_config.build_command();
        }
        // "chat" => final_command = koboldcpp_configure_chat(model_dir, main_command.into_std()),
        &_ => println!("Whoops @ koboldcpp_start"),
    }
    // TODO: Make this print only by a flag
    //println!("{:?}", final_command);
    //
    final_command.stdout(stdout_file);
    final_command.stderr(stderr_file);
    tokio::spawn(async move { koboldcpp_spawn(&mut final_command).await });
}

fn openai_tts_build_prompt(model: &String, input: &String, voice: &String) -> OpenAIVoiceRequest {
    let request = OpenAIVoiceRequest {
        model: model.clone(),
        input: input.clone(),
        voice: voice.clone(),
    };
    return request;
}

fn openai_chat_build_prompt(
    system_prompt: &String,
    user_prompt: &String,
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

// Sends the prompt, and if all goes well
// it returns the response, which is a vector of
// "choices"
pub async fn openai_chat_send_prompt(
    address: &String,
    system_prompt: &String,
    user_prompt: &String,
    temperature: &f32,
    max_tokens: &u32,
) -> Result<OpenAIChatResponse, reqwest::Error> {
    let client = reqwest::Client::new();
    let request = openai_chat_build_prompt(system_prompt, user_prompt, temperature, max_tokens);
    if !check_llm_alive_yet(address).await {
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

pub async fn openai_tts_send_prompt(
    address: &String,
    output_filename: &String,
    model: &String,
    input: &String,
    voice: &String,
) -> Result<File> {
    let client = reqwest::Client::new();
    let request = openai_tts_build_prompt(model, input, voice);
    if !check_llm_alive_yet(address).await {
        println!("Waiting for koboldcpp to be ready...");
    };
    let response = client
        .post(address.to_owned() + "/v1/audio/speech")
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
