package main

func build() func() int {
	return func() int {
		return 7
	}
}
