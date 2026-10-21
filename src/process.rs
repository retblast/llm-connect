use crate::config;
use crate::config::KoboldConfig;
use std::fmt::Display;
use std::{fmt::Formatter, fs::File};
use sysinfo::{ProcessRefreshKind, RefreshKind, System};

#[derive(Debug)]
pub enum Error {
    UncleanExit(tokio::io::Error),
    Configuration(config::Error),
    FailedToStart(tokio::io::Error),
    FailedToCreateLogFile(tokio::io::Error, String),
}

impl From<config::Error> for Error {
    fn from(error: config::Error) -> Self {
        Self::Configuration(error)
    }
}

impl From<tokio::io::Error> for Error {
    fn from(error: tokio::io::Error) -> Self {
        Self::UncleanExit(error)
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UncleanExit(error) => write!(f, "KoboldCPP didn't exit cleanly due to {error}"),
            Self::FailedToStart(error) => write!(f, "KoboldCPP failed to start due to: {error}"),
            Self::Configuration(error) => write!(f, "configuration error due to: {error}"),
            Self::FailedToCreateLogFile(error, kind) => {
                write!(
                    f,
                    "Failed to create the {kind} log file for Koboldcpp due to {error}"
                )
            }
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::FailedToCreateLogFile(error, _)
            | Self::UncleanExit(error)
            | Self::FailedToStart(error) => std::error::Error::source(error),
            Self::Configuration(error) => std::error::Error::source(error),
        }
    }
}

async fn koboldcpp_spawn(command: &mut tokio::process::Command) -> Result<(), Error> {
    loop {
        let mut koboldcpp_process = match command.spawn() {
            Ok(child) => child,
            Err(why) => return Err(Error::FailedToStart(why)),
        };
        let koboldcpp_status = koboldcpp_process.wait().await;

        match koboldcpp_status {
            Ok(_) => {
                println!("Koboldcpp exited successfully.");
                return Ok(());
            }
            Err(why) => return Err(Error::UncleanExit(why)),
        }
    }
}

// Starts koboldcpp
// TODO: make agnostic
pub async fn koboldcpp_start(kobold_config: &KoboldConfig) -> Result<(), Error> {
    // KoboldCPP puts initialization details here, and its last line includes where the http api lies
    let stdout_file = match File::create("koboldcpp_stdout.txt") {
        Ok(file) => file,
        Err(why) => return Err(Error::FailedToCreateLogFile(why, "stdout".to_owned())),
    };
    // And it puts here details about the generation operations
    let stderr_file = match File::create("koboldcpp_stderr.txt") {
        Ok(file) => file,
        Err(why) => return Err(Error::FailedToCreateLogFile(why, "stderr".to_owned())),
    };
    // Build the command
    let mut final_command = kobold_config.build_command()?;
    // TODO: Make this print only by a flag
    //println!("{:?}", final_command);
    //
    final_command.stdout(stdout_file);
    final_command.stderr(stderr_file);

    tokio::spawn(async move { koboldcpp_spawn(&mut final_command).await });

    Ok(())
}

// // Just kills a process by its pid
// pub fn process_killer(pid_to_kill: &u32, process_name: &String) {
//     // Only get processes, without tasks
//     let sys = System::new_with_specifics(
//         RefreshKind::nothing().with_processes(ProcessRefreshKind::everything().without_tasks()),
//     );
//     // Refresh
//     let process = match sys.process(sysinfo::Pid::from_u32(*pid_to_kill)) {
//         Some(process) => process,
//         None => panic!("Something went wrong: {} PID is wrong.", process_name),
//     };
//     match process.kill_and_wait() {
//         Ok(result) => match result {
//             Some(exit_status) => println!("{} exited with status: {}", process_name, exit_status),
//             None => panic!(
//                 "Something happened when trying to wait and kill {}",
//                 process_name
//             ),
//         },
//         Err(error) => panic!(
//             "Something went wrong when trying to kill and wait for {}: {}",
//             process_name, error
//         ),
//     };
// }
