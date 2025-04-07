# Async Batch Engine

An asynchronous batch processing engine for Python and Go. It allows you to process a large number of tasks in batches asynchronously, improving performance by reducing the overhead of individual task processing. Increased throughput.

Use Cases:
- Data Migration(Change Data Capture)/Processing 
- Batch Requests to API

Require:
- need process function support batch process

## Installation

```shell
pip install abatchtaskpool
```

## Usage
### Python
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

### Golang
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
