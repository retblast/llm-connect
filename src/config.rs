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
    model: String,
    wavtokenizer: String,
    voice_refs_dir: String,
}

#[derive(Default)]
pub struct KoboldChatConfig {
    model: String,
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
    pub fn new(model: &str, wavtokenizer: &str, voice_refs_dir: &str) -> Self {
        Self {
            model: model.to_string(),
            wavtokenizer: wavtokenizer.to_string(),
            voice_refs_dir: voice_refs_dir.to_string(),
        }
    }

    fn build_command(&self, main_command: &mut tokio::process::Command) {
        let model = &self.model;
        let wavtokenizer = &self.wavtokenizer;
        let voice_refs_dir = &self.voice_refs_dir;
        main_command
            .arg("--ttsgpu")
            .arg("--ttsmodel")
            .arg(format!("{model}"))
            .arg("--ttswavtokenizer")
            .arg(format!("{wavtokenizer}"))
            .arg("--ttsdir")
            .arg(format!("{voice_refs_dir}"));
    }
}

impl KoboldChatConfig {
    pub fn new(model: &str) -> Self {
        Self {
            model: model.to_string(),
        }
    }

    fn build_command(&self, main_command: &mut tokio::process::Command) {
        let model = &self.model;
        main_command.arg("--model").arg(format!("{model}"));
    }
}
