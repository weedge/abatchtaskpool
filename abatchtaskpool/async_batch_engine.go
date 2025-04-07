package abatchtaskpool

import (
	"sync"
	"time"
)

// Request represents a single request with input data and a channel for the result.
type Request struct {
	InputData  interface{}
	ResultChan chan interface{}
	RequestID  string
}

// AsyncBatchEngine is the main engine for asynchronous batch processing.
type AsyncBatchEngine struct {
	processingFunc func(inputs []interface{}) ([]interface{}, error)
	batchSize      int
	waitTimeout    time.Duration
	numWorkers     int
	requestQueue   chan *Request
	workers        []*BatchWorker
	isRunning      bool
	stopChan       chan struct{}
	waitGroup      *sync.WaitGroup
}

// NewAsyncBatchEngine creates a new AsyncBatchEngine.
func NewAsyncBatchEngine(processingFunc func(inputs []interface{}) ([]interface{}, error), batchSize int, waitTimeout time.Duration, numWorkers int) *AsyncBatchEngine {
	return &AsyncBatchEngine{
		processingFunc: processingFunc,
		batchSize:      batchSize,
		waitTimeout:    waitTimeout,
		numWorkers:     numWorkers,
		requestQueue:   make(chan *Request, 1000), // Buffered channel
		workers:        make([]*BatchWorker, 0, numWorkers),
		isRunning:      false,
		stopChan:       make(chan struct{}),
		waitGroup:      &sync.WaitGroup{},
	}
}

// Start starts the engine and its workers.
func (e *AsyncBatchEngine) Start() {
	if e.isRunning {
		return
	}
	e.isRunning = true
	for i := 0; i < e.numWorkers; i++ {
		worker := NewBatchWorker(i, e.requestQueue, e.batchSize, e.waitTimeout, e.processingFunc, e.waitGroup)
		e.workers = append(e.workers, worker)
		go worker.Run()
	}
}

// Stop stops the engine and its workers.
func (e *AsyncBatchEngine) Stop() {
	if !e.isRunning {
		return
	}
	e.isRunning = false
	for _, worker := range e.workers {
		worker.Stop()
	}
	e.waitGroup.Wait()
	close(e.requestQueue)
	close(e.stopChan)
}

// AddRequest adds a request to the queue and returns a channel to receive the result.
func (e *AsyncBatchEngine) AddRequest(inputData interface{}, requestID string) (chan interface{}, error) {
	if !e.isRunning {
		e.Start()
	}
	resultChan := make(chan interface{}, 1)
	request := &Request{
		InputData:  inputData,
		ResultChan: resultChan,
		RequestID:  requestID,
	}
	e.requestQueue <- request
	return resultChan, nil
}
