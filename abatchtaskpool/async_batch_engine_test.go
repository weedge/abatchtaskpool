package abatchtaskpool

import (
	"fmt"
	"testing"
	"time"
)

func TestAsyncBatchEngine(t *testing.T) {
	engine := NewAsyncBatchEngine(ExampleProcessingFunction, 3, 50*time.Millisecond, 4)
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
			expected := fmt.Sprintf("Processed_task_%d", i)
			if resultStr != expected {
				t.Errorf("Expected result: %s, got: %s", expected, resultStr)
			}
		} else {
			t.Errorf("Unexpected result type: %T", result)
		}
	}

	engine.Stop()
}
