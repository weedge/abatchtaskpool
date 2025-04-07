package abatchtaskpool

import (
	"sync"
	"time"
)

// BatchWorker represents a worker that processes batches of tasks.
type BatchWorker struct {
	workerID       int
	requestQueue   chan *Request
	batchSize      int
	waitTimeout    time.Duration
	processingFunc func(inputs []interface{}) ([]interface{}, error)
	isRunning      bool
	stopChan       chan struct{}
	waitGroup      *sync.WaitGroup
}

// NewBatchWorker creates a new BatchWorker.
func NewBatchWorker(workerID int, requestQueue chan *Request, batchSize int, waitTimeout time.Duration, processingFunc func(inputs []interface{}) ([]interface{}, error), waitGroup *sync.WaitGroup) *BatchWorker {
	return &BatchWorker{
		workerID:       workerID,
		requestQueue:   requestQueue,
		batchSize:      batchSize,
		waitTimeout:    waitTimeout,
		processingFunc: processingFunc,
		isRunning:      true,
		stopChan:       make(chan struct{}),
		waitGroup:      waitGroup,
	}
}

// Run starts the worker's main loop.
func (w *BatchWorker) Run() {
	w.waitGroup.Add(1)
	defer w.waitGroup.Done()
	for w.isRunning {
		batch := w.collectBatch()
		if len(batch) > 0 {
			w.processBatch(batch)
		}
	}
}

// collectBatch collects requests from the queue until the batch size is reached or a timeout occurs.
func (w *BatchWorker) collectBatch() []*Request {
	requests := make([]*Request, 0, w.batchSize)
	timeout := time.After(w.waitTimeout)
	for len(requests) < w.batchSize {
		select {
		case request := <-w.requestQueue:
			requests = append(requests, request)
		case <-timeout:
			return requests
		case <-w.stopChan:
			return requests
		}
	}
	return requests
}

// processBatch processes a batch of requests.
func (w *BatchWorker) processBatch(batch []*Request) {
	inputs := make([]interface{}, len(batch))
	futures := make([]chan interface{}, len(batch))
	for i, req := range batch {
		inputs[i] = req.InputData
		futures[i] = req.ResultChan
	}

	results, err := w.processingFunc(inputs)
	if err != nil {
		for _, future := range futures {
			future <- err
		}
		return
	}

	for i, result := range results {
		futures[i] <- result
	}
}

// Stop stops the worker.
func (w *BatchWorker) Stop() {
	if w.isRunning {
		w.isRunning = false
		close(w.stopChan)
	}
}
