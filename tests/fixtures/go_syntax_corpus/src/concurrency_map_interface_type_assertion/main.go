package main

type subscriber interface {
	Done() <-chan int
}

type registry struct {
	values map[string]chan int
}

func classify(value any, out chan<- int, input <-chan int) map[string]int {
	text := value.(string)
	defer func() {
		out <- len(text)
	}()
	go func(ch <-chan int) {
		out <- <-ch
	}(input)
	select {
	case out <- len(text):
		return map[string]int{text: len(text)}
	case received := <-input:
		return map[string]int{text: received}
	default:
		return map[string]int{}
	}
}
