use std::fmt::Display;
use std::fmt::Formatter;
use std::path::Path;
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
    ) -> Result<Self, Error> {
        // Has to be one or both
        if (tts_config.is_some() || chat_config.is_some()) {
            return Err(Error::EmptyConfig);
        }

        Ok(Self {
            host: host.to_string(),
            port: port.to_owned(),
            tts_config,
            chat_config,
        })
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
    pub fn build_command(&self) -> Result<tokio::process::Command, Error> {
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
            chat_config.build_command(&mut main_command)?;
        }

        if let Some(tts_config) = &self.tts_config {
            tts_config.build_command(&mut main_command)?;
        }

        Ok(main_command)
    }
}

// Make it generic someday
impl KoboldTTSConfig {
    pub fn new(model: &Path, wavtokenizer: &Path, voice_refs_dir: &Path) -> Self {
        Self {
            model: model.to_owned(),
            wavtokenizer: wavtokenizer.to_owned(),
            voice_refs_dir: voice_refs_dir.to_owned(),
        }
    }

    fn build_command(&self, main_command: &mut tokio::process::Command) -> Result<(), Error> {
        let model = &self.model.to_str().ok_or(Error::CantParsePath {
            what: "model".to_string(),
        })?;
        let wavtokenizer = &self.wavtokenizer.to_str().ok_or(Error::CantParsePath {
            what: "wavtokenizer".to_string(),
        })?;
        let voice_refs_dir = &self.voice_refs_dir.to_str().ok_or(Error::CantParsePath {
            what: "voice references directory".to_string(),
        })?;
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
    pub fn new(model: &Path) -> Self {
        Self {
            model: model.to_owned(),
        }
    }

    fn build_command(&self, main_command: &mut tokio::process::Command) -> Result<(), Error> {
        let model = &self.model.to_str().ok_or(Error::CantParsePath {
            what: "model".to_string(),
        })?;
        main_command.arg("--model").arg(format!("{model}"));
        Ok(())
    }
}

#[derive(Debug)]
pub enum Error {
    CantParsePath { what: String },
    EmptyConfig,
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CantParsePath { what } => write!(f, "Can't parse the {what} path"),
            Self::EmptyConfig => write!(f, "No configuration was supplied."),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::CantParsePath { .. } => None,
            Error::EmptyConfig => None,
        }
    }
}
