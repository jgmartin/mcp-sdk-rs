use tokio::process::{Child, Command};
use tokio::sync::mpsc;

use super::io::{handle_stderr, handle_stdin, handle_stdout};

const MESSAGE_BUFFER_SIZE: usize = 100;

#[derive(Debug, thiserror::Error)]
pub enum ProcessError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Process error: {0}")]
    Other(String),
}

pub struct ProcessManager {
    child: Option<Child>,
}

impl Drop for ProcessManager {
    fn drop(&mut self) {
        if let Some(mut child) = self.child.take() {
            // Try to kill the child process when dropping
            let _ = child.start_kill();
            // Note: We can't await here in a Drop, but start_kill()
            // should be sufficient to terminate the process and release file descriptors
        }
    }
}

impl ProcessManager {
    /// Create a new ProcessManager
    pub fn new() -> Self {
        Self { child: None }
    }

    /// Start a new process and return a sender for communicating with it
    pub async fn start_process(
        &mut self,
        command: Command,
        output_tx: mpsc::Sender<String>,
    ) -> Result<mpsc::Sender<String>, ProcessError> {
        // Clean up any existing process first
        self.shutdown().await;

        let child = self.spawn_process(command)?;
        let (process_tx, process_rx) = mpsc::channel::<String>(MESSAGE_BUFFER_SIZE);

        self.setup_io_handlers(child, process_rx, output_tx)?;

        Ok(process_tx)
    }

    /// Spawn a child process with proper stdio configuration
    fn spawn_process(&mut self, mut command: Command) -> Result<Child, ProcessError> {
        log::debug!("spawning process: {:?}", command);

        let child = command
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .kill_on_drop(true)
            .spawn()?;

        Ok(child)
    }

    /// Set up IO handlers for the child process
    fn setup_io_handlers(
        &mut self,
        mut child: Child,
        process_rx: mpsc::Receiver<String>,
        output_tx: mpsc::Sender<String>,
    ) -> Result<(), ProcessError> {
        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| ProcessError::Other("failed to get child stdin".to_string()))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| ProcessError::Other("failed to get child stdout".to_string()))?;
        let stderr = child
            .stderr
            .take()
            .ok_or_else(|| ProcessError::Other("failed to get child stderr".to_string()))?;

        self.child = Some(child);

        tokio::spawn(handle_stdin(stdin, process_rx));
        tokio::spawn(handle_stdout(stdout, output_tx));
        tokio::spawn(handle_stderr(stderr));

        Ok(())
    }

    /// Shutdown the child process gracefully
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
