use std::error::Error;
use tokio::process::{Child, Command};
use tokio::sync::mpsc;

use super::io::{handle_stderr, handle_stdin, handle_stdout};

const MESSAGE_BUFFER_SIZE: usize = 100;

pub struct ProcessManager {
    child: Option<Child>,
}

impl ProcessManager {
    pub fn new() -> Self {
        Self { child: None }
    }

    pub async fn start_process(
        &mut self,
        command: Command,
        output_tx: mpsc::Sender<String>,
    ) -> Result<mpsc::Sender<String>, Box<dyn Error>> {
        let child = self.spawn_process(command)?;
        let (process_tx, process_rx) = mpsc::channel::<String>(MESSAGE_BUFFER_SIZE);

        self.setup_io_handlers(child, process_rx, output_tx)?;

        Ok(process_tx)
    }

    fn spawn_process(&mut self, mut command: Command) -> Result<Child, Box<dyn Error>> {
        log::debug!("spawning process: {:?}", command);

        let child = command
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()?;

        Ok(child)
    }

    fn setup_io_handlers(
        &mut self,
        mut child: Child,
        process_rx: mpsc::Receiver<String>,
        output_tx: mpsc::Sender<String>,
    ) -> Result<(), Box<dyn Error>> {
        let stdin = child.stdin.take().expect("failed to get child stdin");
        let stdout = child.stdout.take().expect("failed to get child stdout");
        let stderr = child.stderr.take().expect("failed to get child stderr");

        self.child = Some(child);

        tokio::spawn(handle_stdin(stdin, process_rx));
        tokio::spawn(handle_stdout(stdout, output_tx));
        tokio::spawn(handle_stderr(stderr));

        Ok(())
    }

    pub async fn shutdown(&mut self) {
        if let Some(mut child) = self.child.take() {
            log::debug!("stopping child process...");
            if let Err(e) = child.kill().await {
                log::error!("failed to stop child process: {}", e);
            }
            if let Err(e) = child.wait().await {
                log::error!("error waiting for child process to exit: {}", e);
            }
            log::debug!("child process stopped");
        }
    }
}
