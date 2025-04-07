// Import necessary items from the library crate
use abatchtaskpool::{AsyncBatchEngine, ProcessingFunction};
use std::sync::Arc;
use std::time::Duration;

// cargo run --package abatchtaskpool --bin abatchtaskpool
#[tokio::main]
async fn main() {
    println!("Starting main application...");

    // Define the processing function for the main application
    let processing_function: ProcessingFunction = Arc::new(|inputs: Vec<String>| {
        Box::pin(async move {
            println!("Main app processing batch of size: {}", inputs.len());
            // Simulate some work
            tokio::time::sleep(Duration::from_millis(150)).await;
            inputs
                .iter()
                .map(|s| format!("MainProcessed: {}", s))
                .collect()
        })
    });

    // Create and start the engine using the library components
    let mut engine = AsyncBatchEngine::new(
        processing_function,
        10, // Different batch size for main app example
        Duration::from_millis(100), // Different timeout
        3,  // More workers
    );
    engine.start().await;

    // Add some requests
    let mut receivers = Vec::new();
    println!("Adding requests...");
    for i in 0..25 { // Add more requests
        match engine.add_request(format!("MainData {}", i), None).await {
            Ok(rx) => receivers.push(rx),
            Err(e) => eprintln!("Failed to add main request {}: {}", i, e),
        }
        // Optional small delay between adding requests
        // tokio::time::sleep(Duration::from_millis(5)).await;
    }
    println!("Finished adding requests.");

    // Collect results
    println!("Waiting for results...");
    let results = futures::future::join_all(receivers.into_iter().map(|rx| async { rx.await })).await;

    println!("\n--- Main App Results ---");
    for (i, result) in results.iter().enumerate() {
        match result {
            Ok(res_str) => println!("Main Request {}: Ok({})", i, res_str),
            Err(e) => println!("Main Request {}: Err({:?})", i, e),
        }
    }
    println!("------------------------\n");

    // Stop the engine
    engine.stop().await;
    println!("Main application finished.");
}
