package main

func makePair[A, B any](a A, b B) string {
	return "pair"
}

func use() string {
	return makePair[int, string](1, "x")
}
