use std::sync::Arc; // Add Arc
use std::time::Duration;
use abatchtaskpool::AsyncBatchEngine;
use futures::FutureExt; // Add FutureExt for .boxed()

// cargo run --package abatchtaskpool --example example
#[tokio::main]
async fn main() {
    // Wrap the function in Arc::new and ensure it returns a BoxFuture
    let processing_fn = Arc::new(|inputs: Vec<String>| {
        async move {
            tokio::time::sleep(Duration::from_secs(1)).await;
            inputs.into_iter().map(|item| format!("Processed_{}", item)).collect()
        }.boxed() // Box the future
    });

    let engine = AsyncBatchEngine::new(
        processing_fn, // Pass the Arc'd and boxed function
        5,
        Duration::from_millis(500),
        2
    );

    let mut tasks = Vec::new();
    for i in 0..10 {
        // Wrap request_id in Some() and await/unwrap the Result
        let task_result = engine.add_request(format!("task_{}", i), Some(format!("req_{}", i))).await;
        match task_result {
            Ok(rx) => tasks.push(rx),
            Err(e) => eprintln!("Failed to add request {}: {}", i, e),
        }
    }

    let results = futures::future::join_all(tasks).await;

    for result in results {
        println!("Result: {:?}", result);
    }

    engine.stop().await;
}
