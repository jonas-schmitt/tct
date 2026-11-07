package main

import (
	"encoding/json"
	"os"
	"path/filepath"
	"testing"
)

func TestRoundTripGateRecordsFirstGeneratedCodecBlocker(t *testing.T) {
	dir := t.TempDir()
	selectionPath := filepath.Join(dir, "selection.json")
	checkerPath := filepath.Join(dir, "checker.json")
	ingestionPath := filepath.Join(dir, "report-ingestion.json")

	selection := corpusSelectionReport{
		ArtifactKind:       selectionArtifactKind,
		Version:            1,
		Tool:               "go_syntax_corpus_select",
		Status:             "ready",
		SourceRoot:         "/usr/local/go/src",
		SourceRevision:     "go1.24.3",
		SelectedCount:      1,
		SelectedBytes:      4873,
		SelectedListSHA256: "selected",
		ExistingManifest:   &selectionExistingManifest{Path: "tests/fixtures/go_syntax_corpus/training-manifest-fifteen-record-v1.json", SHA256: "existing", EntryCount: 14},
		SelectedCandidates: []roundTripCandidate{{
			ManifestPath:    "os/dir_unix.go",
			SourcePath:      "/usr/local/go/src/os/dir_unix.go",
			File:            roundTripFileRef{Path: "/usr/local/go/src/os/dir_unix.go", SHA256: "f40c20d9921fb6353860ac9dcb901362f0b0d98eea53aa8cddab8499501992c5", Bytes: 4873},
			PackageName:     "os",
			SelectionOrder:  1,
			SelectionScore:  4715,
			Selected:        true,
			SelectionReason: "new package",
		}},
	}
	writeTestJSON(t, selectionPath, selection)
	selectionSHA, err := fileSHA256(selectionPath)
	if err != nil {
		t.Fatal(err)
	}
	writeTestJSON(t, checkerPath, corpusCheckReport{
		ArtifactKind:    checkArtifactKind,
		Version:         1,
		Tool:            "go_syntax_corpus_check",
		Status:          "ready",
		SelectionReport: &roundTripReportFile{Path: selectionPath, SHA256: selectionSHA, Bytes: fileSize(t, selectionPath)},
		SourceRoot:      "/usr/local/go/src",
		EntryCount:      1,
		Files: []checkerFileSummary{{
			Path:        "/usr/local/go/src/os/dir_unix.go",
			SHA256:      "f40c20d9921fb6353860ac9dcb901362f0b0d98eea53aa8cddab8499501992c5",
			Bytes:       4873,
			PackageName: "os",
			Constructs:  []string{"function_declaration"},
		}},
		StablePromotionBlockers: []string{"real_source_end_to_end_generated_codec_round_trip"},
	})
	writeTestJSON(t, ingestionPath, reportIngestionManifest{
		CorpusID: "go-stdlib-real-source-syntax-training-corpus-fifteen-record-v1",
		Records:  []reportIngestionRecord{{ManifestPath: "net/nss.go", SourceSHA256: "other"}},
	})

	report, err := buildRoundTripGateReport(roundTripGateOptions{
		selectionReportPath:     selectionPath,
		checkerReportPath:       checkerPath,
		reportIngestionManifest: ingestionPath,
	})
	if err != nil {
		t.Fatal(err)
	}
	if report.Status != roundTripGateStatusBlocked {
		t.Fatalf("expected blocked report, got %s", report.Status)
	}
	if report.SupportedRecordCount != 0 || report.BlockedRecordCount != 1 {
		t.Fatalf("unexpected support/block counts: %+v", report)
	}
	if report.FirstGeneratedCodecBlocker == nil {
		t.Fatal("expected first generated-codec blocker")
	}
	if report.FirstGeneratedCodecBlocker.Kind != "generated_codec_report_ingestion_record_missing" {
		t.Fatalf("unexpected blocker: %+v", report.FirstGeneratedCodecBlocker)
	}
}

func TestRoundTripGatePassesRetainedGeneratedCodecReport(t *testing.T) {
	dir := t.TempDir()
	selectionPath := filepath.Join(dir, "selection.json")
	checkerPath := filepath.Join(dir, "checker.json")
	ingestionPath := filepath.Join(dir, "report-ingestion.json")
	codecReportPath := filepath.Join(dir, "codec-report.json")
	outputPath := filepath.Join(dir, "gate-report.json")

	sourceSHA := "af4e0fa7dfe63018d9808f89a4c511eeb8d773bd6f3b030f1c9592a1e0de7a85"
	sourcePath := "/usr/local/go/src/strings/compare.go"
	selection := corpusSelectionReport{
		ArtifactKind:       selectionArtifactKind,
		Version:            1,
		Tool:               "go_syntax_corpus_select",
		Status:             "ready",
		SourceRoot:         "/usr/local/go/src",
		SourceRevision:     "go1.24.3",
		SelectedCount:      1,
		SelectedBytes:      630,
		SelectedListSHA256: "selected",
		SelectedCandidates: []roundTripCandidate{{
			ManifestPath:    "strings/compare.go",
			SourcePath:      sourcePath,
			File:            roundTripFileRef{Path: sourcePath, SHA256: sourceSHA, Bytes: 630},
			PackageName:     "strings",
			SelectionOrder:  1,
			SelectionScore:  10,
			Selected:        true,
			SelectionReason: "retained generated-codec record",
		}},
	}
	writeTestJSON(t, selectionPath, selection)
	selectionSHA, err := fileSHA256(selectionPath)
	if err != nil {
		t.Fatal(err)
	}
	writeTestJSON(t, checkerPath, corpusCheckReport{
		ArtifactKind:    checkArtifactKind,
		Version:         1,
		Tool:            "go_syntax_corpus_check",
		Status:          "ready",
		SelectionReport: &roundTripReportFile{Path: selectionPath, SHA256: selectionSHA, Bytes: fileSize(t, selectionPath)},
		SourceRoot:      "/usr/local/go/src",
		EntryCount:      1,
		Files: []checkerFileSummary{{
			Path:        sourcePath,
			SHA256:      sourceSHA,
			Bytes:       630,
			PackageName: "strings",
			Constructs:  []string{"function_declaration"},
		}},
	})
	writeTestJSON(t, codecReportPath, generatedCodecReport{
		ArtifactKind: "tct_go_syntax_source_instance_generated_codec_report_v1",
		Status:       "ready",
		Route:        "go_source_instance_projection_contextual_raw_tokens",
		Source:       &generatedCodecSource{Path: sourcePath, SHA256: sourceSHA, Bytes: 630},
		CanonicalRender: map[string]any{
			"gofmt":          "passed",
			"go_parser":      "passed",
			"matches_source": true,
		},
		TCTTokenCount:   3,
		TCTTokensSHA256: "token-sha",
		TCTTokens:       []int{1, 2, 3},
	})
	writeTestJSON(t, ingestionPath, reportIngestionManifest{
		CorpusID: "go-stdlib-real-source-syntax-training-corpus-current-plus",
		Records: []reportIngestionRecord{{
			ManifestPath:        "strings/compare.go",
			SourcePath:          sourcePath,
			SourceSHA256:        sourceSHA,
			GeneratedCodecRoute: "go_source_instance_projection_contextual_raw_tokens",
			ReportPath:          codecReportPath,
		}},
	})

	if err := runRoundTripGate([]string{
		"--selection-report", selectionPath,
		"--checker-report", checkerPath,
		"--report-ingestion-manifest", ingestionPath,
		"--output", outputPath,
	}); err != nil {
		t.Fatal(err)
	}
	if err := runRoundTripGate([]string{
		"--selection-report", selectionPath,
		"--checker-report", checkerPath,
		"--report-ingestion-manifest", ingestionPath,
		"--validate-report", outputPath,
	}); err != nil {
		t.Fatal(err)
	}
	report := readTestJSON[roundTripGateReport](t, outputPath)
	if report.Status != roundTripGateStatusPassed {
		t.Fatalf("expected passed report, got %s", report.Status)
	}
	if report.SupportedRecordCount != 1 || report.BlockedRecordCount != 0 {
		t.Fatalf("unexpected support/block counts: %+v", report)
	}
	if !report.Records[0].Evidence.EncodeDecodeRoundTrip || !report.Records[0].Evidence.GofmtPassed || !report.Records[0].Evidence.GoParserPassed {
		t.Fatalf("expected complete round-trip evidence: %+v", report.Records[0].Evidence)
	}
}

func writeTestJSON(t *testing.T, path string, value any) {
	t.Helper()
	if dir := filepath.Dir(path); dir != "." && dir != "" {
		if err := os.MkdirAll(dir, 0o755); err != nil {
			t.Fatal(err)
		}
	}
	data, err := json.MarshalIndent(value, "", "  ")
	if err != nil {
		t.Fatal(err)
	}
	data = append(data, '\n')
	if err := os.WriteFile(path, data, 0o644); err != nil {
		t.Fatal(err)
	}
}

func readTestJSON[T any](t *testing.T, path string) T {
	t.Helper()
	data, err := os.ReadFile(path)
	if err != nil {
		t.Fatal(err)
	}
	var value T
	if err := json.Unmarshal(data, &value); err != nil {
		t.Fatal(err)
	}
	return value
}

func fileSize(t *testing.T, path string) int64 {
	t.Helper()
	info, err := os.Stat(path)
	if err != nil {
		t.Fatal(err)
	}
	return info.Size()
}
