use futures::future::join_all;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, Mutex};
use tokio::time::timeout;
use uuid::Uuid;

// Define a type alias for the processing function
type ProcessingFunction =
    Arc<dyn Fn(Vec<String>) -> futures::future::BoxFuture<'static, Vec<String>> + Send + Sync>;

// Represents a request with its data and a channel to send the result
struct Request {
    data: String,
    result_tx: tokio::sync::oneshot::Sender<String>,
}

// BatchWorker struct
struct BatchWorker {
    worker_id: usize,
    request_rx: Arc<Mutex<mpsc::Receiver<Request>>>,
    batch_size: usize,
    wait_timeout: Duration,
    processing_function: ProcessingFunction,
}

impl BatchWorker {
    // Constructor for BatchWorker
    fn new(
        worker_id: usize,
        request_rx: Arc<Mutex<mpsc::Receiver<Request>>>,
        batch_size: usize,
        wait_timeout: Duration,
        processing_function: ProcessingFunction,
    ) -> Self {
        BatchWorker {
            worker_id,
            request_rx,
            batch_size,
            wait_timeout,
            processing_function,
        }
    }

    // Worker's main loop
    async fn run(&mut self) {
        println!("Worker {} started.", self.worker_id);
        while let Some(batch) = self.collect_batch().await {
            if !batch.is_empty() {
                println!(
                    "Worker {} processing batch of size {}",
                    self.worker_id,
                    batch.len()
                );
                self.process_batch(batch).await;
            }
            // If batch is empty due to timeout, loop continues to wait for more requests.
            // If collect_batch returns None, the loop terminates.
        }
        println!("Worker {} exiting.", self.worker_id);
    }

    // Collects a batch of requests, returns None if the channel is closed and empty.
    async fn collect_batch(&mut self) -> Option<Vec<Request>> {
        let mut requests = Vec::new();
        let mut request_rx_guard = self.request_rx.lock().await;
        let mut channel_closed = false;

        // Attempt to fill the batch up to batch_size
        while requests.len() < self.batch_size {
            match timeout(self.wait_timeout, request_rx_guard.recv()).await {
                // Timeout occurred
                Err(_) => {
                    break; // Exit collection loop, return any collected requests
                }
                // No timeout, result from recv()
                Ok(result) => {
                    match result {
                        // Request received successfully
                        Some(request) => {
                            requests.push(request);
                        }
                        // Channel closed
                        None => {
                            channel_closed = true;
                            break; // Exit collection loop
                        }
                    }
                }
            }
        }

        // Determine return value
        if requests.is_empty() && channel_closed {
            None // Signal to the worker run loop to exit
        } else {
            Some(requests) // Return collected batch (might be empty if timed out)
        }
    }

    // Processes a batch of requests
    async fn process_batch(&mut self, batch: Vec<Request>) {
        let inputs: Vec<String> = batch.iter().map(|req| req.data.clone()).collect();
        let processing_function = self.processing_function.clone();
        let results_future = processing_function(inputs);
        let results = results_future.await;

        for (request, result) in batch.into_iter().zip(results.into_iter()) {
            let _ = request.result_tx.send(result);
        }
    }
}

// AsyncBatchEngine struct
struct AsyncBatchEngine {
    batch_size: usize,
    wait_timeout: Duration,
    num_workers: usize,
    request_tx: mpsc::Sender<Request>,
    worker_tasks: Vec<tokio::task::JoinHandle<()>>,
}

impl AsyncBatchEngine {
    // Constructor for AsyncBatchEngine
    fn new(
        processing_function: ProcessingFunction,
        batch_size: usize,
        wait_timeout: Duration,
        num_workers: usize,
    ) -> Self {
        let (request_tx, request_rx) = mpsc::channel::<Request>(100);
        let request_rx = Arc::new(Mutex::new(request_rx));

        let mut worker_tasks = Vec::new();
        for i in 0..num_workers {
            let request_rx_clone = request_rx.clone();
            let processing_function_clone = processing_function.clone();
            let mut worker = BatchWorker::new(
                i,
                request_rx_clone,
                batch_size,
                wait_timeout,
                processing_function_clone,
            );
            let task = tokio::spawn(async move {
                worker.run().await;
            });
            worker_tasks.push(task);
        }

        AsyncBatchEngine {
            batch_size,
            wait_timeout,
            num_workers,
            request_tx,
            worker_tasks,
        }
    }

    // Starts the engine
    async fn start(&mut self) {
        // Workers are started in the constructor
        let _ = self.batch_size;
        let _ = self.wait_timeout;
        let _ = self.num_workers;
    }

    // Stops the engine
    async fn stop(mut self) {
        // Drop the request_tx to signal workers to stop
        drop(self.request_tx);
        // Wait for all worker tasks to complete
        join_all(self.worker_tasks.drain(..)).await;
    }

    // Adds a request to the engine
    async fn add_request(&self, input_data: String) -> Result<String, String> {
        let (result_tx, result_rx) = tokio::sync::oneshot::channel::<String>();
        let request = Request {
            data: input_data,
            result_tx,
        };

        self.request_tx
            .send(request)
            .await
            .map_err(|e| e.to_string())?;
        result_rx.await.map_err(|e| e.to_string())
    }
}

#[tokio::main]
async fn main() {
    // Example usage
    let processing_function: ProcessingFunction = Arc::new(|inputs: Vec<String>| {
        Box::pin(async move {
            // Simulate processing
            tokio::time::sleep(Duration::from_millis(100)).await;
            inputs.iter().map(|s| format!("Processed: {}", s)).collect()
        })
    });

    let mut engine = AsyncBatchEngine::new(processing_function, 32, Duration::from_millis(50), 1);
    engine.start().await;

    let result1 = engine.add_request("Data 1".to_string()).await;
    let result2 = engine.add_request("Data 2".to_string()).await;
    let result3 = engine.add_request("Data 3".to_string()).await;

    println!("Result 1: {:?}", result1);
    println!("Result 2: {:?}", result2);
    println!("Result 3: {:?}", result3);

    engine.stop().await;
    println!("Engine stopped.");
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use tokio::time::sleep;

    // Helper function to create a processing function that counts calls
    fn create_counting_processing_function(counter: Arc<AtomicUsize>) -> ProcessingFunction {
        Arc::new(move |inputs: Vec<String>| {
            let counter_clone = counter.clone();
            Box::pin(async move {
                sleep(Duration::from_millis(10)).await;
                counter_clone.fetch_add(1, Ordering::SeqCst);
                inputs.iter().map(|s| format!("Processed: {}", s)).collect()
            })
        })
    }

    #[tokio::test]
    async fn test_add_request() {
        let counter = Arc::new(AtomicUsize::new(0));
        let processing_function = create_counting_processing_function(counter.clone());

        let mut engine =
            AsyncBatchEngine::new(processing_function, 32, Duration::from_millis(50), 1);
        engine.start().await;

        let result = engine.add_request("Test Data".to_string()).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "Processed: Test Data");
        assert_eq!(counter.load(Ordering::SeqCst), 1);

        engine.stop().await;
    }

    #[tokio::test]
    async fn test_batch_processing() {
        let counter = Arc::new(AtomicUsize::new(0));
        let processing_function = create_counting_processing_function(counter.clone());

        let mut engine =
            AsyncBatchEngine::new(processing_function, 2, Duration::from_millis(50), 1);
        engine.start().await;

        let result1 = engine.add_request("Data 1".to_string()).await;
        let result2 = engine.add_request("Data 2".to_string()).await;
        let result3 = engine.add_request("Data 3".to_string()).await;

        assert!(result1.is_ok());
        assert!(result2.is_ok());
        assert!(result3.is_ok());
        assert_eq!(counter.load(Ordering::SeqCst), 2);

        engine.stop().await;
    }

    #[tokio::test]
    async fn test_empty_batch() {
        let counter = Arc::new(AtomicUsize::new(0));
        let processing_function = create_counting_processing_function(counter.clone());

        let mut engine =
            AsyncBatchEngine::new(processing_function, 2, Duration::from_millis(10), 1);
        engine.start().await;

        sleep(Duration::from_millis(50)).await; // Wait for a while to see if any batch is processed
        assert_eq!(counter.load(Ordering::SeqCst), 0);

        engine.stop().await;
    }

    #[tokio::test]
    async fn test_timeout() {
        let counter = Arc::new(AtomicUsize::new(0));
        let processing_function = create_counting_processing_function(counter.clone());

        let mut engine =
            AsyncBatchEngine::new(processing_function, 2, Duration::from_millis(10), 1);
        engine.start().await;

        let _ = engine.add_request("Data 1".to_string()).await;
        sleep(Duration::from_millis(50)).await; // Wait for a while to see if any batch is processed
        assert_eq!(counter.load(Ordering::SeqCst), 1);

        engine.stop().await;
    }

    #[tokio::test]
    async fn test_multiple_workers() {
        let counter = Arc::new(AtomicUsize::new(0));
        let processing_function = create_counting_processing_function(counter.clone());

        let mut engine =
            AsyncBatchEngine::new(processing_function, 2, Duration::from_millis(10), 2);
        engine.start().await;

        let _ = engine.add_request("Data 1".to_string()).await;
        let _ = engine.add_request("Data 2".to_string()).await;
        let _ = engine.add_request("Data 3".to_string()).await;
        let _ = engine.add_request("Data 4".to_string()).await;
        sleep(Duration::from_millis(50)).await; // Wait for a while to see if any batch is processed
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

        let mut engine =
            AsyncBatchEngine::new(processing_function, 2, Duration::from_millis(10), 1);
        engine.start().await;

        let result = engine.add_request("Data 1".to_string()).await;
        assert!(result.is_err());

        engine.stop().await;
    }
}
