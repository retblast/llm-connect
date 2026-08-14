#[derive(Default)]
struct KoboldConfig {
    host: String,
    port: u32,
    tts_config: Option<KoboldTTSConfig>,
    chat_config: Option<KoboldChatConfig>,
}

#[derive(Default)]
struct KoboldTTSConfig {
    model: String,
    wavtokenizer: String,
    voice_refs_dir: String,
}

#[derive(Default)]
struct KoboldChatConfig {
    model: String,
}

impl KoboldConfig {
    fn new(
        mode: String,
        host: String,
        port: u32,
        tts_config: Option<KoboldTTSConfig>,
        chat_config: Option<KoboldChatConfig>,
    ) -> Self {
        // Has to be one or both
        assert!(chat.is_some() || tts.is_some());
        Self {
            host,
            port,
            tts_config,
            chat_config,
        }
    }
    fn build_command(&self) -> tokio::process::Command {
        let host = &self.host;
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

impl Default for KoboldConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".into(),
            port: 5001,
            ..Default::default()
        }
    }
}

// Make it generic someday
impl KoboldTTSConfig {
    fn new(model: String, wavtokenizer: String, voice_refs_dir: String) -> Self {
        Self {
            model,
            wavtokenizer,
            voice_refs_dir,
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
    fn new(model: String) -> Self {
        Self { model }
    }

    fn build_command(&self, main_command: &mut tokio::process::Command) {
        let model = &self.model;
        main_command.arg("--model").arg(format!("{model}"));
    }
}
