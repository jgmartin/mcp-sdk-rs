use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, BufWriter};
use tokio::process::{ChildStderr, ChildStdin, ChildStdout};
use tokio::sync::mpsc;

pub async fn handle_stdin(stdin: ChildStdin, mut process_rx: mpsc::Receiver<String>) {
    let mut writer = BufWriter::new(stdin);
    while let Some(message) = process_rx.recv().await {
        if let Err(e) = write_to_process(&mut writer, &message).await {
            log::error!("Error in stdin handling: {}. Message was: {}", e, message);
            break;
        }
    }
}

pub async fn handle_stdout(stdout: ChildStdout, output_tx: mpsc::Sender<String>) {
    let mut reader = BufReader::new(stdout);
    let mut line = String::new();
    while let Ok(n) = reader.read_line(&mut line).await {
        if should_stop(n) {
            break;
        }

        let trimmed = line.trim().to_string();
        if let Err(e) = output_tx.send(trimmed).await {
            log::error!("Error sending: {}", e);
            break;
        }
        line.clear();
    }
}

pub async fn handle_stderr(stderr: ChildStderr) {
    let mut reader = BufReader::new(stderr);
    let mut line = String::new();
    while let Ok(n) = reader.read_line(&mut line).await {
        if should_stop(n) {
            break;
        }
        line.clear();
    }
}

async fn write_to_process(
    writer: &mut BufWriter<ChildStdin>,
    message: &str,
) -> tokio::io::Result<()> {
    writer.write_all(message.as_bytes()).await?;
    writer.write_all(b"\n").await?;
    writer.flush().await?;
    Ok(())
}

fn should_stop(n: usize) -> bool {
    n == 0
}
