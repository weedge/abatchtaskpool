import logging
import asyncio
from typing import Callable, List, Any, Awaitable, Tuple
from asyncio import Queue, Future
import uuid


class BatchWorker:
    """负责从队列中获取任务并批量处理的 worker 类"""

    def __init__(
        self,
        worker_id: int,
        request_queue: Queue,
        batch_size: int,
        wait_timeout: float,
        processing_function: Callable[[List[Any]], Awaitable[List[Any]]]
    ):
        self.worker_id = worker_id
        self.request_queue = request_queue
        self.batch_size = batch_size
        self.wait_timeout = wait_timeout
        self.processing_function = processing_function
        self.is_running = True

    async def run(self):
        """Worker 主循环，处理批量任务"""
        while self.is_running:
            batch = await self._collect_batch()
            if batch:
                await self._process_batch(batch)

    async def _collect_batch(self) -> List[Tuple[Any, Future]]:
        """从队列中收集任务直到达到批次大小或超时"""
        requests: List[Tuple[Any, Future]] = []
        try:
            while len(requests) < self.batch_size:
                request = await asyncio.wait_for(
                    self.request_queue.get(), timeout=self.wait_timeout
                )
                requests.append(request)
        except asyncio.TimeoutError:
            pass
        return requests

    async def _process_batch(self, batch: List[Tuple[Any, Future]]):
        """处理收集到的批次任务并设置结果"""
        inputs = [req[0] for req in batch]
        futures = [req[1] for req in batch]
        try:
            results = await self.processing_function(inputs)
            for future, result in zip(futures, results):
                if not future.done():
                    future.set_result(result)
        except Exception as e:
            for future in futures:
                if not future.done():
                    future.set_exception(e)

    def stop(self):
        """停止 worker"""
        self.is_running = False


class AsyncBatchEngine:
    """异步批处理引擎，管理任务提交和批量处理"""

    def __init__(
        self,
        processing_function: Callable[[List[Any]], Awaitable[List[Any]]],
        batch_size: int = 32,
        wait_timeout: float = 0.05,
        num_workers: int = 1
    ):
        """
        初始化批处理引擎
        :param processing_function: 批处理函数
        :param batch_size: 批次大小
        :param wait_timeout: 收集任务的超时时间
        :param num_workers: 工作协程数量
        """
        self.processing_function = processing_function
        self.batch_size = batch_size
        self.wait_timeout = wait_timeout
        self.num_workers = num_workers
        self.request_queue: Queue = Queue()
        try:
            self.loop = asyncio.get_running_loop()
        except RuntimeError:
            self.loop = asyncio.new_event_loop()
            asyncio.set_event_loop(self.loop)
        self.workers: List[BatchWorker] = []
        self.worker_tasks: List[asyncio.Task] = []
        self.is_running = False

    async def start(self):
        """启动引擎，创建并运行 worker 任务"""
        if self.is_running:
            return

        self.workers = [
            BatchWorker(
                worker_id=i,
                request_queue=self.request_queue,
                batch_size=self.batch_size,
                wait_timeout=self.wait_timeout,
                processing_function=self.processing_function
            )
            for i in range(self.num_workers)
        ]
        self.worker_tasks = [
            self.loop.create_task(worker.run()) for worker in self.workers
        ]
        self.is_running = True
        await asyncio.sleep(0)  # 确保 worker 启动

    async def stop(self):
        """停止引擎，清理资源"""
        if not self.is_running:
            return

        for worker in self.workers:
            worker.stop()
        for task in self.worker_tasks:
            task.cancel()
            try:
                await task
            except asyncio.CancelledError:
                logging.info("Worker task cancelled.")
        self.is_running = False

    async def add_request(self, input_data: Any, request_id: str = None) -> dict:
        """
        提交任务并返回包含结果的字典
        :param input_data: 输入数据
        :param request_id: 可选的任务 ID
        :return: 包含 request_id 和结果的字典
        """
        if not self.is_running:
            await self.start()

        request_id = request_id or str(uuid.uuid4())
        future = self.loop.create_future()
        self.request_queue.put_nowait((input_data, future))
        result = await future
        return {"request_id": request_id, "feature": result}
