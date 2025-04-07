# Async Batch Engine

An asynchronous batch processing engine for Python.

## Installation

```shell
pip install abatchtaskpool
```

## Usage
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