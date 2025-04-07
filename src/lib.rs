use futures::future::join_all;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, Mutex};
use tokio::time::timeout;
use uuid::Uuid;

// Define a type alias for the processing function - make it public
pub type ProcessingFunction =
    Arc<dyn Fn(Vec<String>) -> futures::future::BoxFuture<'static, Vec<String>> + Send + Sync>;

// Represents a request with its data, request ID, and a channel to send the result - make struct and fields public
#[derive(Debug)] // Add Debug derive for potential use in tests/examples
pub struct Request {
    pub data: String,
    pub request_id: String,
    pub result_tx: tokio::sync::oneshot::Sender<String>,
}

// BatchWorker struct - make it public
pub struct BatchWorker {
    worker_id: usize, // Keep private if only used internally
    request_rx: Arc<Mutex<mpsc::Receiver<Request>>>, // Keep private
    batch_size: usize, // Keep private
    wait_timeout: Duration, // Keep private
    processing_function: ProcessingFunction, // Keep private
}

impl BatchWorker {
    // Constructor for BatchWorker - make it public
    pub fn new(
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

    // Worker's main loop - keep private or make pub if needed externally
    pub async fn run(&mut self) {
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
        }
        println!("Worker {} exiting.", self.worker_id);
    }

    // Collects a batch of requests - keep private
    async fn collect_batch(&mut self) -> Option<Vec<Request>> {
        let mut requests = Vec::new();
        let mut request_rx_guard = self.request_rx.lock().await;
        let mut channel_closed = false;

        while requests.len() < self.batch_size {
            match timeout(self.wait_timeout, request_rx_guard.recv()).await {
                Err(_) => {
                    // Only print timeout if we haven't collected any requests yet in this attempt
                    if requests.is_empty() {
                         println!(
                            "Worker {} timed out waiting for requests (timeout: {:?}).",
                            self.worker_id, self.wait_timeout
                        );
                    }
                    break;
                }
                Ok(result) => {
                    match result {
                        Some(request) => {
                            // Removed verbose logging for received request
                            requests.push(request);
                        }
                        None => {
                            println!(
                                "Worker {} detected channel closed.", // Simplified log
                                self.worker_id
                            );
                            channel_closed = true;
                            break;
                        }
                    }
                }
            }
        }

        if requests.is_empty() && channel_closed {
            None
        } else {
            Some(requests)
        }
    }

    // Processes a batch of requests - keep private
    async fn process_batch(&mut self, batch: Vec<Request>) {
        let inputs: Vec<String> = batch.iter().map(|req| req.data.clone()).collect();
        let processing_function = self.processing_function.clone();

        // Execute the processing function
        let results = processing_function(inputs).await;

        // Send results back
        // Ensure the number of results matches the number of requests
        if batch.len() == results.len() {
            for (request, result) in batch.into_iter().zip(results.into_iter()) {
                if request.result_tx.send(result).is_err() {
                    // Log error if sending fails (receiver might have been dropped)
                    println!("Worker {} failed to send result for request {}", self.worker_id, request.request_id);
                }
            }
        } else {
             // Log error if result count doesn't match batch size
             println!("Worker {} error: Mismatch between batch size ({}) and result size ({})", self.worker_id, batch.len(), results.len());
             // Attempt to send an error message back or handle appropriately
             let error_message = format!("Processing error: batch size {} != result size {}", batch.len(), results.len());
             for request in batch {
                 let _ = request.result_tx.send(error_message.clone());
             }
        }
    }
}

// AsyncBatchEngine struct - make it public
pub struct AsyncBatchEngine {
    _batch_size: usize, // Prefix with _ to silence dead_code warning (used in new())
    _wait_timeout: Duration, // Prefix with _ to silence dead_code warning (used in new())
    num_workers: usize, // Keep private
    request_tx: mpsc::Sender<Request>, // Keep private
    worker_tasks: Vec<tokio::task::JoinHandle<()>>, // Keep private
}

impl AsyncBatchEngine {
    // Constructor for AsyncBatchEngine - make it public
    pub fn new(
        processing_function: ProcessingFunction,
        batch_size: usize,
        wait_timeout: Duration,
        num_workers: usize,
    ) -> Self {
        let (request_tx, request_rx) = mpsc::channel::<Request>(100); // Consider making buffer size configurable
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
            _batch_size: batch_size,
            _wait_timeout: wait_timeout,
            num_workers,
            request_tx,
            worker_tasks,
        }
    }

    // Starts the engine - make public (currently just logs)
    pub async fn start(&mut self) {
        // Workers are already started in new()
        // This method could be used for additional setup or logging if needed.
        println!("AsyncBatchEngine started with {} workers.", self.num_workers);
    }

    // Stops the engine - make public
    pub async fn stop(mut self) {
        println!("Stopping AsyncBatchEngine...");
        // Drop the sender to signal workers no more requests are coming
        drop(self.request_tx);
        // Wait for all worker tasks to complete
        join_all(self.worker_tasks.drain(..)).await;
        println!("AsyncBatchEngine stopped.");
    }

    // Adds a request to the engine - make public
    pub async fn add_request(
        &self,
        input_data: String,
        request_id: Option<String>,
    ) -> Result<tokio::sync::oneshot::Receiver<String>, String> {
        let (result_tx, result_rx) = tokio::sync::oneshot::channel::<String>();
        let final_request_id = request_id.filter(|id| !id.is_empty())
                                        .unwrap_or_else(|| Uuid::new_v4().to_string());
        let request = Request {
            data: input_data,
            request_id: final_request_id,
            result_tx,
        };

        match self.request_tx.send(request).await {
            Ok(_) => Ok(result_rx),
            Err(e) => {
                // Log the error before returning it
                eprintln!("Failed to send request to worker channel: {}", e);
                Err(format!("Failed to send request: {}", e))
            }
        }
    }
}

// Add a simple test within the library itself (optional but good practice)
#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    fn create_test_processing_function(counter: Arc<AtomicUsize>) -> ProcessingFunction {
        Arc::new(move |inputs: Vec<String>| {
            let counter_clone = counter.clone();
            Box::pin(async move {
                tokio::time::sleep(Duration::from_millis(10)).await;
                counter_clone.fetch_add(inputs.len(), Ordering::SeqCst); // Increment by batch size
                inputs.iter().map(|s| format!("LibProcessed: {}", s)).collect()
            })
        })
    }

    #[tokio::test]
    async fn test_lib_engine_simple() {
        let counter = Arc::new(AtomicUsize::new(0));
        let processing_function = create_test_processing_function(counter.clone());
        let mut engine = AsyncBatchEngine::new(processing_function, 2, Duration::from_millis(50), 1);
        engine.start().await;

        let rx1 = engine.add_request("LibData1".to_string(), None).await.unwrap();
        let rx2 = engine.add_request("LibData2".to_string(), None).await.unwrap();

        let res1 = rx1.await.unwrap();
        let res2 = rx2.await.unwrap();

        assert_eq!(res1, "LibProcessed: LibData1");
        assert_eq!(res2, "LibProcessed: LibData2");
        assert_eq!(counter.load(Ordering::SeqCst), 2); // Processed 2 items

        engine.stop().await;
    }
}
