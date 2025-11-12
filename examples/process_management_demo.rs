/// This example demonstrates the fixed process management in mcp-sdk-rs
/// It shows how child processes are now properly cleaned up, preventing file descriptor leaks
use mcp_sdk_rs::process::ProcessManager;
use std::time::Instant;
use tokio::time::{sleep, Duration};

#[tokio::main]
async fn main() -> Result<(), mcp_sdk_rs::process::ProcessError> {
    env_logger::init();
    
    println!("Process Management Demo - Testing File Descriptor Cleanup");
    println!("=========================================================");
    
    let start_time = Instant::now();
    
    // Simulate multiple process manager lifecycles 
    // This would previously cause file descriptor leaks
    for i in 1..=10 {
        println!("Starting process manager cycle {}", i);
        
        let mut pm = ProcessManager::new();
        let (output_tx, mut output_rx) = tokio::sync::mpsc::channel(100);
        
        // Create a command that will produce some output
        let mut command = tokio::process::Command::new("echo");
        command.arg(format!("Hello from process {}", i));
        
        // Start the process
        let process_tx = pm.start_process(command, output_tx).await?;
        
        // Send a message to the process (echo will just ignore stdin for this example)
        let _ = process_tx.send("test message".to_string()).await;
        
        // Read the output
        if let Some(output) = output_rx.recv().await {
            println!("  Received: {}", output);
        }
        
        // Give the process a moment to complete
        sleep(Duration::from_millis(50)).await;
        
        // Explicitly shutdown (this is now properly implemented)
        pm.shutdown().await;
        
        println!("  Process manager cycle {} completed", i);
        
        // Small delay between cycles
        sleep(Duration::from_millis(100)).await;
    }
    
    let elapsed = start_time.elapsed();
    println!("All process cycles completed successfully in {:?}", elapsed);
    println!("If you were monitoring file descriptors, they should remain stable!");
    
    // Wait a bit more to ensure all cleanup is done
    sleep(Duration::from_millis(500)).await;
    
    println!("Demo completed successfully. No file descriptor leaks should have occurred.");
    
    Ok(())
}