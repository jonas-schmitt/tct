package main

func build(values ...int) int {
	return total(values...)
}

func total(values ...int) int {
	return len(values)
}
