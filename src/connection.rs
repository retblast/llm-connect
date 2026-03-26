// All about connections
use std::process::Command;

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

// Returns the PID of the running koboldcpp instance
pub fn koboldcpp_start(mode: &String, model_dir: &String, voice_refs_dir: &String) -> u32 {
    let mut main_command = Command::new("koboldcpp");
    main_command
        .arg("--gpulayers")
        .arg("-1")
        .arg("--threads")
        .arg("16")
        .arg("--usevulkan");
    let mut final_command = Command::new("ls");
    match mode.as_str() {
        "tts" => final_command = koboldcpp_configure_tts(model_dir, voice_refs_dir, main_command),
        &_ => println!("Whoops @ koboldcpp_start"),
    }
    println!("{:?}", final_command);
    final_command.spawn().expect("Starting koboldcpp failed");

    //TODO: Get the PID
    return 0;
}

fn koboldcpp_build_prompt(
    mode: &String,
    system_prompt: &String,
    user_prompt: &String,
    temperature: &f32,
    max_tokens: &u32,
) {
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
