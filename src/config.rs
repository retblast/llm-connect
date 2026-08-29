use std::fmt::Display;
use std::fmt::Formatter;
use std::path::PathBuf;

// think of giving it a default someday
#[derive(Default)]
pub struct KoboldConfig {
    host: String,
    port: u32,
    tts_config: Option<KoboldTTSConfig>,
    chat_config: Option<KoboldChatConfig>,
}

#[derive(Default)]
pub struct KoboldTTSConfig {
    model: PathBuf,
    wavtokenizer: PathBuf,
    voice_refs_dir: PathBuf,
}

#[derive(Default)]
pub struct KoboldChatConfig {
    model: PathBuf,
}

impl KoboldConfig {
    pub fn new(
        host: &str,
        port: &u32,
        tts_config: Option<KoboldTTSConfig>,
        chat_config: Option<KoboldChatConfig>,
    ) -> Self {
        // Has to be one or both
        assert!(tts_config.is_some() || chat_config.is_some());
        Self {
            host: host.to_string(),
            port: port.to_owned(),
            tts_config,
            chat_config,
        }
    }

    // koboldcpp's --host arg prepends http://
    // handle this ourselves
    fn sanitize_host(host: &str) -> String {
        match host.strip_prefix("http://") {
            Some(stripped_host) => stripped_host.to_string(),
            // TODO: Learn about that specialsauce Rust patterns thingy :P
            // the one with the ||
            None => match host.strip_prefix("https://") {
                Some(stripped_host) => stripped_host.to_string(),
                None => host.to_owned(),
            },
        }
    }

    // Build command, return it to store
    pub fn build_command(&self) -> tokio::process::Command {
        let host = KoboldConfig::sanitize_host(&self.host);
        let port = &self.port;
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
            .arg("--usevulkan");
        main_command.kill_on_drop(true);

        if let Some(chat_config) = &self.chat_config {
            chat_config.build_command(&mut main_command);
        }

        if let Some(tts_config) = &self.tts_config {
            tts_config.build_command(&mut main_command);
        }

        main_command
    }
}

// Make it generic someday
impl KoboldTTSConfig {
    pub fn new(model: &PathBuf, wavtokenizer: &PathBuf, voice_refs_dir: &PathBuf) -> Self {
        Self {
            model: model.to_owned(),
            wavtokenizer: wavtokenizer.to_owned(),
            voice_refs_dir: voice_refs_dir.to_owned(),
        }
    }

    fn build_command(
        &self,
        main_command: &mut tokio::process::Command,
    ) -> Result<(), LlmConfigError> {
        let model = &self.model.to_str().ok_or(LlmConfigError::CantParsePath)?;
        let wavtokenizer = &self
            .wavtokenizer
            .to_str()
            .ok_or(LlmConfigError::CantParsePath)?;
        let voice_refs_dir = &self
            .voice_refs_dir
            .to_str()
            .ok_or(LlmConfigError::CantParsePath)?;
        main_command
            .arg("--ttsgpu")
            .arg("--ttsmodel")
            .arg(format!("{model}"))
            .arg("--ttswavtokenizer")
            .arg(format!("{wavtokenizer}"))
            .arg("--ttsdir")
            .arg(format!("{voice_refs_dir}"));
        Ok(())
    }
}

impl KoboldChatConfig {
    pub fn new(model: &PathBuf) -> Self {
        Self {
            model: model.to_owned(),
        }
    }

    fn build_command(
        &self,
        main_command: &mut tokio::process::Command,
    ) -> Result<(), LlmConfigError> {
        let model = &self.model.to_str().ok_or(LlmConfigError::CantParsePath)?;
        main_command.arg("--model").arg(format!("{model}"));
        Ok(())
    }
}

#[derive(Debug)]
enum LlmConfigError {
    CantParsePath,
}

impl Display for LlmConfigError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CantParsePath => write!(f, "Can't parse path"),
        }
    }
}
