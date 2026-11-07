package main

func build(value any) int {
	var count = 1
	switch value.(type) {
	case int:
		count = 7
	}
	return count
}
