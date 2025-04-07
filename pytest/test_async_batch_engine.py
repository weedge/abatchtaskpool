import random
from typing import Any, List
import unittest
import asyncio
from abatchtaskpool.async_batch_engine import AsyncBatchEngine

"""
python -m unittest pytest/test_async_batch_engine.py
"""


async def example_processing_function(inputs: List[Any]) -> List[Any]:
    """示例批处理函数"""
    s_time = random.randint(1, 3)
    print(f"Processing batch: {inputs} sleeping for {s_time} seconds")
    await asyncio.sleep(s_time)  # 模拟处理耗时
    return [f"Processed_{item}" for item in inputs]


class TestAsyncBatchEngine(unittest.IsolatedAsyncioTestCase):
    async def asyncSetUp(self):
        self.engine = AsyncBatchEngine(
            processing_function=example_processing_function,
            batch_size=3,
            wait_timeout=0.05,
            num_workers=4,
        )

    async def test_add_request(self):
        # 启动引擎
        await self.engine.start()

        # 提交任务
        tasks = [
            self.engine.add_request(f"task_{i}", f"req_{i}")
            for i in range(10)
        ]

        # 获取结果
        results = await asyncio.gather(*tasks)

        # 验证结果
        for i, result in enumerate(results):
            self.assertEqual(result, {"request_id": f"req_{i}", "feature": f"Processed_task_{i}"})

        # 停止引擎
        await self.engine.stop()

    async def test_start_stop(self):
        # 启动引擎
        await self.engine.start()

        # 检查引擎是否正在运行
        self.assertTrue(self.engine.is_running)

        # 停止引擎
        await self.engine.stop()

        # 检查引擎是否已停止
        self.assertFalse(self.engine.is_running)
