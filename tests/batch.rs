use abatchtaskpool::{AsyncBatchEngine, ProcessingFunction}; // Import from crate
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;

// Helper function: Define directly in the test file
fn create_counting_processing_function(counter: Arc<AtomicUsize>) -> ProcessingFunction {
    Arc::new(move |inputs: Vec<String>| {
        let counter_clone = counter.clone();
        // Use Box::pin as the ProcessingFunction type expects a pinned future
        Box::pin(async move {
            sleep(Duration::from_millis(10)).await;
            counter_clone.fetch_add(1, Ordering::SeqCst);
            inputs.iter().map(|s| format!("Processed: {}", s)).collect()
        })
    })
}

// cargo test --package abatchtaskpool --test batch
// --- Test Functions ---

#[tokio::test]
async fn test_add_request() {
    let counter = Arc::new(AtomicUsize::new(0));
    let processing_function = create_counting_processing_function(counter.clone());

    let mut engine = AsyncBatchEngine::new(processing_function, 32, Duration::from_millis(50), 1);
    engine.start().await;

    let rx = engine.add_request("Test Data".to_string(), None).await;
    assert!(rx.is_ok());
    let result = rx.unwrap().await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "Processed: Test Data");
    // Wait a bit for processing to likely complete before checking counter
    sleep(Duration::from_millis(60)).await;
    assert_eq!(counter.load(Ordering::SeqCst), 1);

    engine.stop().await;
}

#[tokio::test]
async fn test_batch_processing() {
    let counter = Arc::new(AtomicUsize::new(0));
    let processing_function = create_counting_processing_function(counter.clone());

    let mut engine = AsyncBatchEngine::new(processing_function, 2, Duration::from_millis(50), 1);
    engine.start().await;

    let rx1 = engine.add_request("Data 1".to_string(), None).await;
    let rx2 = engine.add_request("Data 2".to_string(), None).await;
    let rx3 = engine.add_request("Data 3".to_string(), None).await;

    assert!(rx1.is_ok());
    assert!(rx2.is_ok());
    assert!(rx3.is_ok());

    // Await results
    let res1 = rx1.unwrap().await;
    let res2 = rx2.unwrap().await;
    let res3 = rx3.unwrap().await;

    assert!(res1.is_ok());
    assert!(res2.is_ok());
    assert!(res3.is_ok());

    // Wait a bit for processing to likely complete before checking counter
    sleep(Duration::from_millis(60)).await;
    // The counter might be 2 or 3 depending on batching and timing.
    // With batch size 2, requests 1&2 form a batch, request 3 forms another.
    // So the processing function should be called twice.
    assert_eq!(counter.load(Ordering::SeqCst), 2);

    engine.stop().await;
}

#[tokio::test]
async fn test_empty_batch() {
    let counter = Arc::new(AtomicUsize::new(0));
    let processing_function = create_counting_processing_function(counter.clone());

    let mut engine = AsyncBatchEngine::new(processing_function, 2, Duration::from_millis(10), 1);
    engine.start().await;

    sleep(Duration::from_millis(50)).await; // Wait for a while to see if any batch is processed
    assert_eq!(counter.load(Ordering::SeqCst), 0);

    engine.stop().await;
}

#[tokio::test]
async fn test_timeout() {
    let counter = Arc::new(AtomicUsize::new(0));
    let processing_function = create_counting_processing_function(counter.clone());

    let mut engine = AsyncBatchEngine::new(processing_function, 2, Duration::from_millis(10), 1);
    engine.start().await;

    let rx = engine.add_request("Data 1".to_string(), None).await;
    assert!(rx.is_ok());
    let _ = rx.unwrap().await; // Await the result

    sleep(Duration::from_millis(50)).await; // Wait for timeout processing
    assert_eq!(counter.load(Ordering::SeqCst), 1); // Should be processed

    engine.stop().await;
}

#[tokio::test]
async fn test_multiple_workers() {
    let counter = Arc::new(AtomicUsize::new(0));
    let processing_function = create_counting_processing_function(counter.clone());

    let mut engine = AsyncBatchEngine::new(processing_function, 2, Duration::from_millis(10), 2);
    engine.start().await;

    let rx1 = engine.add_request("Data 1".to_string(), None).await;
    let rx2 = engine.add_request("Data 2".to_string(), None).await;
    let rx3 = engine.add_request("Data 3".to_string(), None).await;
    let rx4 = engine.add_request("Data 4".to_string(), None).await;

    assert!(rx1.is_ok());
    assert!(rx2.is_ok());
    assert!(rx3.is_ok());
    assert!(rx4.is_ok());

    // Await all results concurrently by awaiting the receivers inside join!
    // Wrap each await in an async block to help type inference.
    let (res1, res2, res3, res4) = tokio::join!(
        async { rx1.unwrap().await },
        async { rx2.unwrap().await },
        async { rx3.unwrap().await },
        async { rx4.unwrap().await }
    );

    // Check results (optional, but good practice)
    assert!(res1.is_ok());
    assert!(res2.is_ok());
    assert!(res3.is_ok());
    assert!(res4.is_ok());

    sleep(Duration::from_millis(60)).await; // Wait for processing
                                            // With batch size 2, requests 1&2 form a batch, 3&4 form another.
                                            // Processing function called twice.
    assert_eq!(counter.load(Ordering::SeqCst), 2);

    engine.stop().await;
}

#[tokio::test]
async fn test_processing_function_error() {
    let processing_function: ProcessingFunction = Arc::new(|_inputs: Vec<String>| {
        Box::pin(async move {
            panic!("Simulated error in processing function");
        })
    });

    let mut engine = AsyncBatchEngine::new(processing_function, 2, Duration::from_millis(10), 1);
    engine.start().await;

    let rx = engine.add_request("Data 1".to_string(), None).await;
    assert!(rx.is_ok());
    let result = rx.unwrap().await;
    // The error now comes from the receiver await, not the add_request call itself
    assert!(result.is_err()); // Error because the processing function panicked

    engine.stop().await;
}
