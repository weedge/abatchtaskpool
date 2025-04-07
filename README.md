# Async Batch Engine

An asynchronous batch processing engine for Python and Go. It allows you to process a large number of tasks in batches asynchronously, improving performance by reducing the overhead of individual task processing. Increased throughput.

Use Cases:
- Data Migration(Change Data Capture)/Processing 
- Batch Requests to API

Require:
- need process function support batch process


# Usage
## Python
Installation
```shell
pip install abatchtaskpool
```
```python
from typing import Any, List
import asyncio

from abatchtaskpool.async_batch_engine import AsyncBatchEngine

async def example_processing_function(inputs: List[Any]) -> List[Any]:
    """示例批处理函数"""
    await asyncio.sleep(1)  # 模拟处理耗时
    return [f"Processed_{item}" for item in inputs]

async def main():
    # 创建引擎实例
    engine = AsyncBatchEngine(
        processing_function=example_processing_function,
        batch_size=5,
        wait_timeout=0.5,
        num_workers=2
    )

    # 提交任务并获取结果
    tasks = [
        engine.add_request(f"task_{i}", f"req_{i}")
        for i in range(10)
    ]
    results = await asyncio.gather(*tasks)
    
    # 输出结果
    for result in results:
        print(f"Result: {result}")

    # 停止引擎
    await engine.stop()

if __name__ == "__main__":
    asyncio.run(main())

```

## Golang
```go
package main

import (
	"fmt"
	"math/rand"
	"time"
	"abatchtaskpool"
)

func ExampleProcessingFunction(inputs []interface{}) ([]interface{}, error) {
	sTime := rand.Intn(3) + 1
	fmt.Printf("Processing batch: %v sleeping for %d seconds\n", inputs, sTime)
	time.Sleep(time.Duration(sTime) * time.Second)
	results := make([]interface{}, len(inputs))
	for i, item := range inputs {
		results[i] = fmt.Sprintf("Processed_%v", item)
	}
	return results, nil
}

func main() {
	engine := abatchtaskpool.NewAsyncBatchEngine(ExampleProcessingFunction, 3, 50*time.Millisecond, 4)
	engine.Start()

	numTasks := 10
	results := make([]chan interface{}, numTasks)
	for i := 0; i < numTasks; i++ {
		resultChan, _ := engine.AddRequest(fmt.Sprintf("task_%d", i), fmt.Sprintf("req_%d", i))
		results[i] = resultChan
	}

	for i, resultChan := range results {
		result := <-resultChan
		if resultStr, ok := result.(string); ok {
			fmt.Printf("Result: %s\n", resultStr)
		} else {
			fmt.Printf("Unexpected result type: %T\n", result)
		}
	}

	engine.Stop()
}
```

## Rust
```shell
cargo run --package abatchtaskpool --example demo
cargo test --package abatchtaskpool --test batch
cargo run --package abatchtaskpool --bin abatchtaskpool
```

`cargo run --package abatchtaskpool --example example`
```rust
use std::sync::Arc; // Add Arc
use std::time::Duration;
use abatchtaskpool::AsyncBatchEngine;
use futures::FutureExt; // Add FutureExt for .boxed()

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
```