use tokio::time::{sleep, Duration};

/// Test to verify that child processes are properly cleaned up
/// and file descriptors are not leaked
#[tokio::test]
async fn test_process_cleanup() {
    use mcp_sdk_rs::process::ProcessManager;
    
    let initial_fd_count = get_fd_count().unwrap_or(0);
    println!("Initial FD count: {}", initial_fd_count);
    
    // Create multiple ProcessManagers to simulate multiple sessions
    for i in 0..5 {
        let mut pm = ProcessManager::new();
        let (output_tx, _output_rx) = tokio::sync::mpsc::channel(100);
        
        // Create a simple command that will run briefly
        let mut command = tokio::process::Command::new("echo");
        command.arg("Hello from process").arg(i.to_string());
        
        let _process_tx = pm.start_process(command, output_tx).await.expect("Failed to start process");
        
        // Let the process run briefly
        sleep(Duration::from_millis(100)).await;
        
        // The ProcessManager will be dropped here, triggering cleanup
        pm.shutdown().await;
    }
    
    // Give some time for cleanup
    sleep(Duration::from_millis(500)).await;
    
    let final_fd_count = get_fd_count().unwrap_or(0);
    println!("Final FD count: {}", final_fd_count);
    
    // The file descriptor count should not have grown significantly
    // Allow for some variance due to other system activity
    assert!(final_fd_count <= initial_fd_count + 10, 
           "File descriptor leak detected: initial={}, final={}", 
           initial_fd_count, final_fd_count);
}

/// Test that demonstrates the issue exists without the fix
#[tokio::test]
async fn test_session_resource_management() {
    use mcp_sdk_rs::session::Session;
    use tokio::sync::mpsc;
    use std::sync::Arc;
    
    let initial_fd_count = get_fd_count().unwrap_or(0);
    println!("Initial FD count for session test: {}", initial_fd_count);
    
    // Create multiple sessions to test resource management
    for i in 0..3 {
        let (sender, receiver) = mpsc::unbounded_channel();
        let mut command = tokio::process::Command::new("sleep");
        command.arg("0.1"); // Sleep for 100ms
        
        let session = Session::Local {
            handler: None,
            command,
            receiver: Arc::new(tokio::sync::Mutex::new(receiver)),
            sender: Arc::new(sender),
        };
        
        // Start the session
        session.start().await.expect("Failed to start session");
        
        // Give it time to process
        sleep(Duration::from_millis(200)).await;
        
        println!("Completed session {}", i);
    }
    
    // Give time for cleanup
    sleep(Duration::from_millis(1000)).await;
    
    let final_fd_count = get_fd_count().unwrap_or(0);
    println!("Final FD count for session test: {}", final_fd_count);
    
    // Check that we haven't leaked too many file descriptors
    assert!(final_fd_count <= initial_fd_count + 15, 
           "Session file descriptor leak detected: initial={}, final={}", 
           initial_fd_count, final_fd_count);
}

/// Get the current number of open file descriptors for this process
/// This is a Unix-specific implementation
#[cfg(unix)]
fn get_fd_count() -> Option<usize> {
    use std::fs;
    
    // On Unix systems, /proc/self/fd contains links to all open file descriptors
    if let Ok(entries) = fs::read_dir("/proc/self/fd") {
        Some(entries.count())
    } else {
        // Fallback for macOS and other Unix systems
        None
    }
}

#[cfg(not(unix))]
fn get_fd_count() -> Option<usize> {
    // On non-Unix systems, we can't easily count file descriptors
    None
}

/// Stress test to verify no file descriptor leaks under load
#[tokio::test]
async fn test_no_fd_leak_under_load() {
    let initial_fd_count = get_fd_count().unwrap_or(0);
    println!("Initial FD count for stress test: {}", initial_fd_count);
    
    let mut task_count = 0;
    
    // Create multiple ProcessManagers sequentially to avoid Send issues
    for _ in 0..10 {
        for i in 0..5 {
            let mut pm = mcp_sdk_rs::process::ProcessManager::new();
            let (output_tx, _output_rx) = tokio::sync::mpsc::channel(100);
            
            let mut command = tokio::process::Command::new("echo");
            command.arg(format!("test-{}", i));
            
            if let Ok(_process_tx) = pm.start_process(command, output_tx).await {
                sleep(Duration::from_millis(10)).await;
                pm.shutdown().await;
            }
            
            task_count += 1;
        }
    }
    
    println!("Completed {} tasks", task_count);
    
    // Give time for cleanup
    sleep(Duration::from_millis(500)).await;
    
    let final_fd_count = get_fd_count().unwrap_or(0);
    println!("Final FD count for stress test: {}", final_fd_count);
    
    // Under stress, we should still not leak file descriptors
    assert!(final_fd_count <= initial_fd_count + 20, 
           "Stress test file descriptor leak detected: initial={}, final={}", 
           initial_fd_count, final_fd_count);
}