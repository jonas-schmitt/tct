package main

type Counter struct {
	Value int
}

func build() Counter {
	return Counter{Value: 7}
}
