package main

import "testing"

func TestSanitizeModuleDirName(t *testing.T) {
	got := sanitizeModuleDirName("golang.org/x/tools@v0.38.0")
	want := "golang.org__x__tools__v0.38.0"
	if got != want {
		t.Fatalf("sanitizeModuleDirName = %q, want %q", got, want)
	}
}

func TestPathHasPrefix(t *testing.T) {
	if !pathHasPrefix("prometheus/client.go", nil) {
		t.Fatal("empty prefixes should include all paths")
	}
	if !pathHasPrefix("prometheus/client.go", []string{"prometheus"}) {
		t.Fatal("expected prometheus/client.go to match prefix prometheus")
	}
	if pathHasPrefix("api/client.go", []string{"prometheus"}) {
		t.Fatal("expected api/client.go to miss prefix prometheus")
	}
}

func TestValidatePinsRejectsEmptyModules(t *testing.T) {
	pins := stressPinsDocument{
		ArtifactKind: stressPinsArtifactKind,
		Version:      stressVersion,
	}
	if err := validatePins(&pins); err == nil {
		t.Fatal("expected empty modules to fail")
	}
}

func TestSelfTestOffline(t *testing.T) {
	if err := runSelfTest(); err != nil {
		t.Fatalf("runSelfTest: %v", err)
	}
}
