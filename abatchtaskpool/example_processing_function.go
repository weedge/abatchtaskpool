package abatchtaskpool

import (
	"fmt"
	"math/rand"
	"time"
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
