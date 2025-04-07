use abatchtaskpool::{AsyncBatchEngine, ProcessingFunction}; // Import from the library crate
use std::sync::Arc;
use std::time::Duration;

// Define the processing function matching the ProcessingFunction type
fn create_example_processing_function() -> ProcessingFunction {
    Arc::new(|inputs: Vec<String>| {
        Box::pin(async move {
            println!("Processing batch of size: {}", inputs.len());
            tokio::time::sleep(Duration::from_millis(100)).await; // Shorter sleep for example
            inputs
                .iter()
                .map(|s| format!("Processed: {}", s))
                .collect()
        })
    })
}

// cargo run --package abatchtaskpool --example demo

#[tokio::main]
async fn main() {
    let processing_function = create_example_processing_function();
    let mut engine = AsyncBatchEngine::new(
        processing_function,
        5,                     // batch_size
        Duration::from_millis(50), // wait_timeout (reduced for quicker batching)
        2,                     // num_workers
    );
    engine.start().await; // Start the engine

    let mut receivers = Vec::new();
    for i in 0..10 {
        // add_request now returns a Result
        match engine.add_request(format!("Data {}", i), Some(format!("req_{}", i))).await {
            Ok(rx) => receivers.push(rx),
            Err(e) => eprintln!("Failed to add request {}: {}", i, e),
        }
        // Small sleep to allow requests to queue and potentially trigger timeout batching
        // tokio::time::sleep(Duration::from_millis(10)).await;
    }

    // Collect results concurrently
    let results = futures::future::join_all(receivers.into_iter().map(|rx| async { rx.await })).await;

    println!("\n--- Results ---");
    for (i, result) in results.iter().enumerate() {
        match result {
            Ok(res_str) => println!("Request {}: Ok({})", i, res_str),
            Err(e) => println!("Request {}: Err({:?})", i, e), // RecvError
        }
    }
    println!("---------------\n");

    // Stop the engine gracefully
    engine.stop().await;
}
