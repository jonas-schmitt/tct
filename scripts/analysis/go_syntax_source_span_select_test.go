package main

import (
	"os"
	"path/filepath"
	"reflect"
	"testing"
)

func TestGoSyntaxSourceSpanSelectPrefersConcurrencySpan(t *testing.T) {
	dir := t.TempDir()
	sourcePath := filepath.Join(dir, "signal.go")
	source := `package signal

type stringer interface {
	String() string
}

func small() {}

func NotifyContext(signals ...Signal) {
	ch := make(chan Signal, 1)
	go func() {
		select {
		case ch <- signals[0]:
		default:
		}
	}()
}
`
	if err := os.WriteFile(sourcePath, []byte(source), 0o644); err != nil {
		t.Fatal(err)
	}

	report, err := buildGoSyntaxSpanSelectionReport(goSyntaxSpanSelectionConfig{
		SourcePath:       sourcePath,
		ManifestPath:     "os/signal/signal.go",
		MaxSpanBytes:     512,
		TargetConstructs: defaultGoSyntaxSpanTargetConstructs,
		SelectionNote:    "test",
	})
	if err != nil {
		t.Fatal(err)
	}
	if report.Status != "ready" {
		t.Fatalf("expected ready report, got %s", report.Status)
	}
	if report.SelectedCandidate == nil {
		t.Fatal("expected selected candidate")
	}
	if report.SelectedCandidate.Name != "NotifyContext" {
		t.Fatalf("expected NotifyContext span, got %s", report.SelectedCandidate.Name)
	}
	expectedTargets := []string{
		"channel_type",
		"comm_clause",
		"ellipsis",
		"function_literal",
		"go_statement",
		"select_statement",
		"send_statement",
	}
	if !reflect.DeepEqual(report.SelectedCandidate.TargetConstructs, expectedTargets) {
		t.Fatalf("unexpected target constructs: %v", report.SelectedCandidate.TargetConstructs)
	}
	if report.ClaimBoundaries.ArenaIRPromotion {
		t.Fatal("source-span selection must not promote arena IR")
	}
	if report.ClaimBoundaries.GeneratedCodecChanged {
		t.Fatal("source-span selection must not claim generated-codec changes")
	}
}

func TestGoSyntaxSourceSpanSelectReportsBlockedWhenBudgetIsTooSmall(t *testing.T) {
	dir := t.TempDir()
	sourcePath := filepath.Join(dir, "signal.go")
	source := `package signal

func NotifyContext(signals ...Signal) {
	ch := make(chan Signal, 1)
	go func() {
		select {
		case ch <- signals[0]:
		default:
		}
	}()
}
`
	if err := os.WriteFile(sourcePath, []byte(source), 0o644); err != nil {
		t.Fatal(err)
	}

	report, err := buildGoSyntaxSpanSelectionReport(goSyntaxSpanSelectionConfig{
		SourcePath:       sourcePath,
		ManifestPath:     "os/signal/signal.go",
		MaxSpanBytes:     8,
		TargetConstructs: defaultGoSyntaxSpanTargetConstructs,
		SelectionNote:    "test",
	})
	if err != nil {
		t.Fatal(err)
	}
	if report.Status != "blocked" {
		t.Fatalf("expected blocked report, got %s", report.Status)
	}
	if report.SelectedCandidate != nil {
		t.Fatalf("too-small budget should not select candidate: %+v", report.SelectedCandidate)
	}
	if len(report.Candidates) != 1 {
		t.Fatalf("expected one candidate, got %d", len(report.Candidates))
	}
}
