package main

import (
	"encoding/json"
	"os"
	"path/filepath"
	"reflect"
	"strings"
	"testing"
)

func TestGeneratedCodecIntakeRecordsRendererBlockerForPreferredCandidate(t *testing.T) {
	dir := t.TempDir()
	selectionPath := filepath.Join(dir, "selection.json")
	existingPath := filepath.Join(dir, "existing.json")
	metadataRoot := filepath.Join(dir, "metadata")
	reportIngestionPath := filepath.Join(dir, "report-ingestion.json")

	writeTestJSON(t, selectionPath, selectionReport{
		ArtifactKind:       "tct_go_syntax_corpus_selection_report_v1",
		Version:            1,
		Tool:               "go_syntax_corpus_select",
		Status:             "ready",
		SourceRoot:         filepath.Join(runtimeGOROOTForTest(), "src"),
		SourceRevision:     "go1.24.3",
		SelectedCount:      2,
		SelectedBytes:      721,
		SelectedListSHA256: "selection",
		SelectedCandidates: []goSourceCandidate{
			{
				ManifestPath:    "fmt/errors.go",
				SourcePath:      filepath.Join(runtimeGOROOTForTest(), "src/fmt/errors.go"),
				SourceURI:       "https://example.invalid/fmt/errors.go",
				File:            sourceFileRef{Path: filepath.Join(runtimeGOROOTForTest(), "src/fmt/errors.go"), SHA256: "fmtsha", Bytes: 361},
				PackageName:     "fmt",
				SelectionOrder:  1,
				SelectionScore:  11,
				Selected:        true,
				SelectionReason: "new package",
			},
			{
				ManifestPath:     "internal/coverage/uleb128/uleb128.go",
				SourcePath:       filepath.Join(runtimeGOROOTForTest(), "src/internal/coverage/uleb128/uleb128.go"),
				SourceURI:        "https://example.invalid/internal/coverage/uleb128/uleb128.go",
				File:             sourceFileRef{Path: filepath.Join(runtimeGOROOTForTest(), "src/internal/coverage/uleb128/uleb128.go"), SHA256: "ulebsha", Bytes: 360},
				PackageName:      "uleb128",
				UniqueConstructs: []string{"for_statement", "if_statement"},
				ShapeSHA256:      "shape",
				SelectionOrder:   2,
				SelectionScore:   10,
				Selected:         true,
				SelectionReason:  "new package",
			},
		},
	})
	writeTestJSON(t, existingPath, corpusManifest{
		CorpusID: "go-stdlib-real-source-syntax-training-corpus-eight-record-v1",
		Entries:  []corpusManifestEntry{{ManifestPath: "strings/compare.go"}},
	})
	if err := os.MkdirAll(metadataRoot, 0o755); err != nil {
		t.Fatal(err)
	}
	writeTestJSON(t, reportIngestionPath, reportIngestionManifest{
		CorpusID: "go-stdlib-real-source-syntax-training-corpus-eight-record-v1",
		Records:  []reportIngestionRecord{{ManifestPath: "strings/compare.go"}},
	})

	report, err := buildIntakeReport(intakeOptions{
		selectionReportPath:     selectionPath,
		preferredManifestPath:   "internal/coverage/uleb128/uleb128.go",
		existingManifestPath:    existingPath,
		metadataRoot:            metadataRoot,
		reportIngestionManifest: reportIngestionPath,
	})
	if err != nil {
		t.Fatal(err)
	}
	if report.Status != intakeStatusBlocked {
		t.Fatalf("expected blocked intake status, got %s", report.Status)
	}
	if report.SelectedCandidate.ManifestPath != "internal/coverage/uleb128/uleb128.go" {
		t.Fatalf("unexpected selected candidate: %s", report.SelectedCandidate.ManifestPath)
	}
	if report.SelectionPolicy.SelectedBy != "preferred_selected_candidate" {
		t.Fatalf("unexpected selection policy: %s", report.SelectionPolicy.SelectedBy)
	}
	if report.CurrentEvidence.AlreadyInExistingManifest {
		t.Fatal("new selected source must not be treated as existing corpus evidence")
	}
	if report.CurrentEvidence.CheckedMetadataFixtureFound {
		t.Fatal("missing checked metadata fixture should be recorded")
	}
	if report.CurrentEvidence.GeneratedCodecReportFound {
		t.Fatal("missing generated-codec report should be recorded")
	}
	expectedKinds := []string{
		"checked_metadata_fixture_missing",
		"generated_codec_renderer_abstraction_missing",
		"report_ingestion_manifest_record_missing",
		"training_corpus_manifest_record_missing",
		"unsupported_syntax_constructs_for_reusable_intake",
	}
	if !reflect.DeepEqual(blockerKinds(report.Blockers), expectedKinds) {
		t.Fatalf("unexpected blockers: %v", blockerKinds(report.Blockers))
	}
	if report.CurrentEvidence.SyntaxConstructCoverage.UnsupportedConstructCount != 2 {
		t.Fatalf("unexpected unsupported construct count: %d", report.CurrentEvidence.SyntaxConstructCoverage.UnsupportedConstructCount)
	}
	if report.CheckedSourceTargetPath != "tests/fixtures/go_syntax_corpus/src/internal/coverage/uleb128/uleb128.go" {
		t.Fatalf("unexpected checked source target: %s", report.CheckedSourceTargetPath)
	}
	if !strings.Contains(report.ValidationCommandTemplate[1], "--existing-manifest "+existingPath) {
		t.Fatalf("selection command should preserve the current existing manifest: %s", report.ValidationCommandTemplate[1])
	}
}

func TestGeneratedCodecIntakeRecordsRendererGapFromCheckedMetadata(t *testing.T) {
	dir := t.TempDir()
	selectionPath := filepath.Join(dir, "selection.json")
	existingPath := filepath.Join(dir, "existing.json")
	metadataRoot := filepath.Join(dir, "metadata")
	reportIngestionPath := filepath.Join(dir, "report-ingestion.json")
	inventoryPath := filepath.Join(dir, "inventory.json")
	selectedSourcePath := "tests/fixtures/go_syntax_corpus/src/os/signal/signal.go"
	existingSourcePath := "tests/fixtures/go_syntax_corpus/src/strings/compare.go"

	writeTestJSON(t, selectionPath, selectionReport{
		ArtifactKind:       "tct_go_syntax_corpus_selection_report_v1",
		Version:            1,
		Tool:               "go_syntax_corpus_select",
		Status:             "ready",
		SourceRoot:         filepath.Join(runtimeGOROOTForTest(), "src"),
		SourceRevision:     "go1.24.3",
		SelectedCount:      1,
		SelectedBytes:      8480,
		SelectedListSHA256: "selection",
		SelectedCandidates: []goSourceCandidate{
			{
				ManifestPath:    "os/signal/signal.go",
				SourcePath:      "",
				SourceURI:       "https://example.invalid/os/signal/signal.go",
				File:            sourceFileRef{SHA256: "signalsha", Bytes: 8480},
				PackageName:     "signal",
				ShapeSHA256:     "shape",
				SelectionOrder:  1,
				SelectionScore:  100,
				Selected:        true,
				SelectionReason: "new package",
			},
		},
	})
	writeTestJSON(t, existingPath, corpusManifest{
		CorpusID: "go-stdlib-real-source-syntax-training-corpus-nine-record-v1",
		Entries:  []corpusManifestEntry{{ManifestPath: "strings/compare.go"}},
	})
	writeTestJSON(t, filepath.Join(metadataRoot, "selected", "syntax-metadata-v0.json"), map[string]any{
		"source": map[string]string{"path": selectedSourcePath},
		"rules": []map[string]string{
			{"name": "File", "node_kind": "File"},
			{"name": "FuncDecl", "node_kind": "FuncDecl"},
			{"name": "ChanType", "node_kind": "ChanType"},
			{"name": "SelectStmt", "node_kind": "SelectStmt"},
		},
	})
	writeTestJSON(t, filepath.Join(metadataRoot, "existing", "syntax-metadata-v0.json"), map[string]any{
		"source": map[string]string{"path": existingSourcePath},
		"rules": []map[string]string{
			{"name": "File", "node_kind": "File"},
			{"name": "FuncDecl", "node_kind": "FuncDecl"},
		},
	})
	writeTestJSON(t, reportIngestionPath, reportIngestionManifest{
		CorpusID: "go-stdlib-real-source-syntax-training-corpus-nine-record-v1",
		Records: []reportIngestionRecord{{
			ManifestPath:        "strings/compare.go",
			SourcePath:          existingSourcePath,
			GeneratedCodecRoute: "go_strings_compare_source_file_syntax_v0",
			ReportPath:          "target/tct-syntax-real-source-demo/go-strings-compare-generated-codec-demo.json",
		}},
	})
	writeTestInventory(t, inventoryPath, "File", "FuncDecl")

	report, err := buildIntakeReport(intakeOptions{
		selectionReportPath:          selectionPath,
		preferredManifestPath:        "os/signal/signal.go",
		existingManifestPath:         existingPath,
		metadataRoot:                 metadataRoot,
		reportIngestionManifest:      reportIngestionPath,
		rendererCodegenInventoryPath: inventoryPath,
	})
	if err != nil {
		t.Fatal(err)
	}
	gap := report.CurrentEvidence.GeneratedCodecRendererGap
	if !gap.Analyzed {
		t.Fatal("checked metadata should enable renderer gap analysis")
	}
	if gap.ExistingReportIngestionRecordCount != 1 || gap.ExistingGeneratedCodecMetadataCount != 1 {
		t.Fatalf("unexpected existing coverage counts: %+v", gap)
	}
	if gap.RendererCodegenCoverageInventoryPath != filepath.ToSlash(inventoryPath) {
		t.Fatalf("unexpected inventory coverage path: %+v", gap)
	}
	if gap.SelectedMetadataRuleCount != 4 || gap.CoveredSelectedMetadataRuleCount != 2 {
		t.Fatalf("unexpected selected coverage counts: %+v", gap)
	}
	expectedMissing := []string{"ChanType", "SelectStmt"}
	if !reflect.DeepEqual(gap.MissingGeneratedCodecRendererRules, expectedMissing) {
		t.Fatalf("unexpected renderer gaps: %v", gap.MissingGeneratedCodecRendererRules)
	}
	if !strings.Contains(report.NextConcreteTask, "ChanType, SelectStmt") {
		t.Fatalf("next task should name renderer gaps: %s", report.NextConcreteTask)
	}
	var rendererReason string
	for _, blocker := range report.Blockers {
		if blocker.Kind == "generated_codec_renderer_abstraction_missing" {
			rendererReason = blocker.Reason
		}
	}
	if !strings.Contains(rendererReason, "ChanType, SelectStmt") {
		t.Fatalf("renderer blocker should name precise gaps: %s", rendererReason)
	}
}

func TestGeneratedCodecIntakeUsesInventoryRendererCoverage(t *testing.T) {
	dir := t.TempDir()
	selectionPath := filepath.Join(dir, "selection.json")
	existingPath := filepath.Join(dir, "existing.json")
	metadataRoot := filepath.Join(dir, "metadata")
	reportIngestionPath := filepath.Join(dir, "report-ingestion.json")
	inventoryPath := filepath.Join(dir, "inventory.json")
	selectedSourcePath := "tests/fixtures/go_syntax_corpus/src/os/signal/signal.go"
	existingSourcePath := "tests/fixtures/go_syntax_corpus/src/strings/compare.go"

	writeTestJSON(t, selectionPath, selectionReport{
		ArtifactKind:       "tct_go_syntax_corpus_selection_report_v1",
		Version:            1,
		Tool:               "go_syntax_corpus_select",
		Status:             "ready",
		SourceRoot:         filepath.Join(runtimeGOROOTForTest(), "src"),
		SourceRevision:     "go1.24.3",
		SelectedCount:      1,
		SelectedBytes:      8480,
		SelectedListSHA256: "selection",
		SelectedCandidates: []goSourceCandidate{{
			ManifestPath:    "os/signal/signal.go",
			SourceURI:       "https://example.invalid/os/signal/signal.go",
			File:            sourceFileRef{SHA256: "signalsha", Bytes: 8480},
			PackageName:     "signal",
			ShapeSHA256:     "shape",
			SelectionOrder:  1,
			SelectionScore:  100,
			Selected:        true,
			SelectionReason: "new package",
		}},
	})
	writeTestJSON(t, existingPath, corpusManifest{
		CorpusID: "go-stdlib-real-source-syntax-training-corpus-nine-record-v1",
		Entries:  []corpusManifestEntry{{ManifestPath: "strings/compare.go"}},
	})
	writeTestJSON(t, filepath.Join(metadataRoot, "selected", "syntax-metadata-v0.json"), map[string]any{
		"source": map[string]string{"path": selectedSourcePath},
		"rules": []map[string]string{
			{"name": "File", "node_kind": "File"},
			{"name": "FuncDecl", "node_kind": "FuncDecl"},
			{"name": "ChanType", "node_kind": "ChanType"},
			{"name": "SelectStmt", "node_kind": "SelectStmt"},
		},
	})
	writeTestJSON(t, filepath.Join(metadataRoot, "existing", "syntax-metadata-v0.json"), map[string]any{
		"source": map[string]string{"path": existingSourcePath},
		"rules": []map[string]string{
			{"name": "File", "node_kind": "File"},
			{"name": "FuncDecl", "node_kind": "FuncDecl"},
		},
	})
	writeTestJSON(t, reportIngestionPath, reportIngestionManifest{
		CorpusID: "go-stdlib-real-source-syntax-training-corpus-nine-record-v1",
		Records: []reportIngestionRecord{{
			ManifestPath:        "strings/compare.go",
			SourcePath:          existingSourcePath,
			GeneratedCodecRoute: "go_strings_compare_source_file_syntax_v0",
			ReportPath:          "target/tct-syntax-real-source-demo/go-strings-compare-generated-codec-demo.json",
		}},
	})
	writeTestInventory(t, inventoryPath, "File", "FuncDecl", "ChanType")

	report, err := buildIntakeReport(intakeOptions{
		selectionReportPath:          selectionPath,
		preferredManifestPath:        "os/signal/signal.go",
		existingManifestPath:         existingPath,
		metadataRoot:                 metadataRoot,
		reportIngestionManifest:      reportIngestionPath,
		rendererCodegenInventoryPath: inventoryPath,
	})
	if err != nil {
		t.Fatal(err)
	}
	gap := report.CurrentEvidence.GeneratedCodecRendererGap
	if gap.RendererReadinessSource != "go_ast_raw_tree_renderer_codegen_coverage_and_dispatch_audit" {
		t.Fatalf("unexpected renderer readiness source: %+v", gap)
	}
	if gap.CoveredSelectedMetadataRuleCount != 3 {
		t.Fatalf("inventory renderer coverage should cover one selected missing rule: %+v", gap)
	}
	expectedMissing := []string{"SelectStmt"}
	if !reflect.DeepEqual(gap.MissingGeneratedCodecRendererRules, expectedMissing) {
		t.Fatalf("unexpected renderer gaps after inventory coverage: %v", gap.MissingGeneratedCodecRendererRules)
	}
	if strings.Contains(report.NextConcreteTask, "ChanType") {
		t.Fatalf("next task should not keep inventory-supported rule: %s", report.NextConcreteTask)
	}
}

func TestGeneratedCodecIntakeStopsRendererBlockerWhenInventoryCoverageIsComplete(t *testing.T) {
	dir := t.TempDir()
	selectionPath := filepath.Join(dir, "selection.json")
	existingPath := filepath.Join(dir, "existing.json")
	metadataRoot := filepath.Join(dir, "metadata")
	reportIngestionPath := filepath.Join(dir, "report-ingestion.json")
	inventoryPath := filepath.Join(dir, "inventory.json")
	selectedSourcePath := "tests/fixtures/go_syntax_corpus/src/os/signal/signal.go"
	existingSourcePath := "tests/fixtures/go_syntax_corpus/src/strings/compare.go"

	writeTestJSON(t, selectionPath, selectionReport{
		ArtifactKind:       "tct_go_syntax_corpus_selection_report_v1",
		Version:            1,
		Tool:               "go_syntax_corpus_select",
		Status:             "ready",
		SourceRoot:         filepath.Join(runtimeGOROOTForTest(), "src"),
		SourceRevision:     "go1.24.3",
		SelectedCount:      1,
		SelectedBytes:      8480,
		SelectedListSHA256: "selection",
		SelectedCandidates: []goSourceCandidate{{
			ManifestPath:    "os/signal/signal.go",
			SourceURI:       "https://example.invalid/os/signal/signal.go",
			File:            sourceFileRef{SHA256: "signalsha", Bytes: 8480},
			PackageName:     "signal",
			ShapeSHA256:     "shape",
			SelectionOrder:  1,
			SelectionScore:  100,
			Selected:        true,
			SelectionReason: "new package",
		}},
	})
	writeTestJSON(t, existingPath, corpusManifest{
		CorpusID: "go-stdlib-real-source-syntax-training-corpus-nine-record-v1",
		Entries:  []corpusManifestEntry{{ManifestPath: "strings/compare.go"}},
	})
	writeTestJSON(t, filepath.Join(metadataRoot, "selected", "syntax-metadata-v0.json"), map[string]any{
		"source": map[string]string{"path": selectedSourcePath},
		"rules": []map[string]string{
			{"name": "File", "node_kind": "File"},
			{"name": "FuncDecl", "node_kind": "FuncDecl"},
			{"name": "Ellipsis", "node_kind": "Ellipsis"},
		},
	})
	writeTestJSON(t, filepath.Join(metadataRoot, "existing", "syntax-metadata-v0.json"), map[string]any{
		"source": map[string]string{"path": existingSourcePath},
		"rules": []map[string]string{
			{"name": "File", "node_kind": "File"},
			{"name": "FuncDecl", "node_kind": "FuncDecl"},
		},
	})
	writeTestJSON(t, reportIngestionPath, reportIngestionManifest{
		CorpusID: "go-stdlib-real-source-syntax-training-corpus-nine-record-v1",
		Records: []reportIngestionRecord{{
			ManifestPath:        "strings/compare.go",
			SourcePath:          existingSourcePath,
			GeneratedCodecRoute: "go_strings_compare_source_file_syntax_v0",
			ReportPath:          "target/tct-syntax-real-source-demo/go-strings-compare-generated-codec-demo.json",
		}},
	})
	writeTestInventory(t, inventoryPath, "File", "FuncDecl", "Ellipsis")

	report, err := buildIntakeReport(intakeOptions{
		selectionReportPath:          selectionPath,
		preferredManifestPath:        "os/signal/signal.go",
		existingManifestPath:         existingPath,
		metadataRoot:                 metadataRoot,
		reportIngestionManifest:      reportIngestionPath,
		rendererCodegenInventoryPath: inventoryPath,
	})
	if err != nil {
		t.Fatal(err)
	}
	if report.CurrentEvidence.GeneratedCodecRendererGap.MissingGeneratedCodecRendererRuleCount != 0 {
		t.Fatalf("inventory renderer coverage should be complete: %+v", report.CurrentEvidence.GeneratedCodecRendererGap)
	}
	expectedKinds := []string{
		"generated_codec_source_route_missing",
		"report_ingestion_manifest_record_missing",
		"training_corpus_manifest_record_missing",
	}
	if !reflect.DeepEqual(blockerKinds(report.Blockers), expectedKinds) {
		t.Fatalf("unexpected blockers after complete inventory renderer coverage: %v", blockerKinds(report.Blockers))
	}
	if report.CurrentEvidence.GeneratedCodecSourceRoute.Found {
		t.Fatal("inventory rule coverage alone must not prove selected-source route support")
	}
	if strings.Contains(report.NextConcreteTask, "renderer coverage") {
		t.Fatalf("next task should move to report production: %s", report.NextConcreteTask)
	}
	if !strings.Contains(report.NextConcreteTask, "source-route report support") {
		t.Fatalf("next task should name the route-level generated-codec blocker: %s", report.NextConcreteTask)
	}
}

func TestGeneratedCodecIntakeRequiresReportIngestionForSelectedSourceRouteSupport(t *testing.T) {
	dir := t.TempDir()
	selectionPath := filepath.Join(dir, "selection.json")
	existingPath := filepath.Join(dir, "existing.json")
	metadataRoot := filepath.Join(dir, "metadata")
	reportIngestionPath := filepath.Join(dir, "report-ingestion.json")
	inventoryPath := filepath.Join(dir, "inventory.json")
	selectedSourcePath := "tests/fixtures/go_syntax_corpus/src/os/signal/signal.go"
	existingSourcePath := "tests/fixtures/go_syntax_corpus/src/strings/compare.go"

	writeTestJSON(t, selectionPath, selectionReport{
		ArtifactKind:       "tct_go_syntax_corpus_selection_report_v1",
		Version:            1,
		Tool:               "go_syntax_corpus_select",
		Status:             "ready",
		SourceRoot:         filepath.Join(runtimeGOROOTForTest(), "src"),
		SourceRevision:     "go1.24.3",
		SelectedCount:      1,
		SelectedBytes:      8480,
		SelectedListSHA256: "selection",
		SelectedCandidates: []goSourceCandidate{{
			ManifestPath:    "os/signal/signal.go",
			SourceURI:       "https://example.invalid/os/signal/signal.go",
			File:            sourceFileRef{SHA256: "signalsha", Bytes: 8480},
			PackageName:     "signal",
			ShapeSHA256:     "shape",
			SelectionOrder:  1,
			SelectionScore:  100,
			Selected:        true,
			SelectionReason: "new package",
		}},
	})
	writeTestJSON(t, existingPath, corpusManifest{
		CorpusID: "go-stdlib-real-source-syntax-training-corpus-nine-record-v1",
		Entries:  []corpusManifestEntry{{ManifestPath: "strings/compare.go"}},
	})
	writeTestJSON(t, filepath.Join(metadataRoot, "selected", "syntax-metadata-v0.json"), map[string]any{
		"source": map[string]string{"path": selectedSourcePath},
		"rules": []map[string]string{
			{"name": "File", "node_kind": "File"},
			{"name": "FuncDecl", "node_kind": "FuncDecl"},
		},
	})
	writeTestJSON(t, filepath.Join(metadataRoot, "existing", "syntax-metadata-v0.json"), map[string]any{
		"source": map[string]string{"path": existingSourcePath},
		"rules": []map[string]string{
			{"name": "File", "node_kind": "File"},
			{"name": "FuncDecl", "node_kind": "FuncDecl"},
		},
	})
	writeTestJSON(t, reportIngestionPath, reportIngestionManifest{
		CorpusID: "go-stdlib-real-source-syntax-training-corpus-nine-record-v1",
		Records: []reportIngestionRecord{{
			ManifestPath:        "strings/compare.go",
			SourcePath:          existingSourcePath,
			GeneratedCodecRoute: "go_strings_compare_source_file_syntax_v0",
			ReportPath:          "target/tct-syntax-real-source-demo/go-strings-compare-generated-codec-demo.json",
		}},
	})
	writeTestInventory(t, inventoryPath, "File", "FuncDecl")

	report, err := buildIntakeReport(intakeOptions{
		selectionReportPath:          selectionPath,
		preferredManifestPath:        "os/signal/signal.go",
		existingManifestPath:         existingPath,
		metadataRoot:                 metadataRoot,
		reportIngestionManifest:      reportIngestionPath,
		rendererCodegenInventoryPath: inventoryPath,
	})
	if err != nil {
		t.Fatal(err)
	}
	if report.CurrentEvidence.GeneratedCodecSourceRoute.Found {
		t.Fatalf("selected-source route support must require report-ingestion evidence: %+v", report.CurrentEvidence.GeneratedCodecSourceRoute)
	}
	expectedKinds := []string{
		"generated_codec_source_route_missing",
		"report_ingestion_manifest_record_missing",
		"training_corpus_manifest_record_missing",
	}
	if !reflect.DeepEqual(blockerKinds(report.Blockers), expectedKinds) {
		t.Fatalf("unexpected blockers without report-ingestion route support: %v", blockerKinds(report.Blockers))
	}
	if !strings.Contains(report.NextConcreteTask, "source-route report support") {
		t.Fatalf("next task should require report-ingestion source-route evidence: %s", report.NextConcreteTask)
	}
}

func TestGeneratedCodecIntakePrefersWholeSourceMetadataOverSpan(t *testing.T) {
	dir := t.TempDir()
	metadataRoot := filepath.Join(dir, "metadata")
	selectedSourcePath := "tests/fixtures/go_syntax_corpus/src/os/signal/signal.go"
	spanPath := filepath.Join(metadataRoot, "os_signal_notify_context_source_span", "syntax-metadata-v0.json")
	sourcePath := filepath.Join(metadataRoot, "os_signal_source_file", "syntax-metadata-v0.json")

	writeTestJSON(t, spanPath, map[string]any{
		"source": map[string]string{"path": selectedSourcePath},
		"rules": []map[string]string{
			{"name": "File", "node_kind": "File"},
			{"name": "FuncDecl", "node_kind": "FuncDecl"},
		},
	})
	writeTestJSON(t, sourcePath, map[string]any{
		"source": map[string]string{"path": selectedSourcePath},
		"rules": []map[string]string{
			{"name": "File", "node_kind": "File"},
			{"name": "FuncDecl", "node_kind": "FuncDecl"},
			{"name": "ChanType", "node_kind": "ChanType"},
		},
	})

	found, ok, err := findMetadataFixture(metadataRoot, selectedSourcePath)
	if err != nil {
		t.Fatal(err)
	}
	if !ok {
		t.Fatal("expected metadata fixture match")
	}
	if found != filepath.ToSlash(sourcePath) {
		t.Fatalf("expected whole-source metadata fixture, got %s", found)
	}
}

func TestGeneratedCodecIntakeRecordsSourceInstanceReportEvidence(t *testing.T) {
	dir := t.TempDir()
	selectionPath := filepath.Join(dir, "selection.json")
	existingPath := filepath.Join(dir, "existing.json")
	metadataRoot := filepath.Join(dir, "metadata")
	reportIngestionPath := filepath.Join(dir, "report-ingestion.json")
	inventoryPath := filepath.Join(dir, "inventory.json")
	selectedSourcePath := "tests/fixtures/go_syntax_corpus/src/net/lookup_plan9.go"
	padTokenAbsent := true

	writeTestJSON(t, selectionPath, selectionReport{
		ArtifactKind:       "tct_go_syntax_corpus_selection_report_v1",
		Version:            1,
		Tool:               "go_syntax_corpus_select",
		Status:             "ready",
		SourceRoot:         filepath.Join(runtimeGOROOTForTest(), "src"),
		SourceRevision:     "go1.24.3",
		SelectedCount:      1,
		SelectedBytes:      10127,
		SelectedListSHA256: "selection",
		SelectedCandidates: []goSourceCandidate{{
			ManifestPath:    "net/lookup_plan9.go",
			SourcePath:      filepath.Join(runtimeGOROOTForTest(), "src/net/lookup_plan9.go"),
			SourceURI:       "https://example.invalid/net/lookup_plan9.go",
			File:            sourceFileRef{SHA256: "4abcbf57125304d52839b0cfc13fbd0c2ae5b10c7f60374009e85ac47252bde2", Bytes: 10127},
			PackageName:     "net",
			ShapeSHA256:     "shape",
			SelectionOrder:  1,
			SelectionScore:  100,
			Selected:        true,
			SelectionReason: "new package",
		}},
	})
	writeTestJSON(t, existingPath, corpusManifest{
		CorpusID: "go-stdlib-real-source-syntax-training-corpus-eleven-record-v1",
		Entries:  []corpusManifestEntry{{ManifestPath: "net/lookup_plan9.go"}},
	})
	writeTestJSON(t, filepath.Join(metadataRoot, "selected", "syntax-metadata-v0.json"), map[string]any{
		"source": map[string]string{"path": selectedSourcePath},
		"rules": []map[string]string{
			{"name": "File", "node_kind": "File"},
			{"name": "FuncDecl", "node_kind": "FuncDecl"},
		},
	})
	writeTestJSON(t, reportIngestionPath, reportIngestionManifest{
		CorpusID: "go-stdlib-real-source-syntax-training-corpus-eleven-record-v1",
		Records: []reportIngestionRecord{{
			ManifestPath:                     "net/lookup_plan9.go",
			SourcePath:                       selectedSourcePath,
			SourceSHA256:                     "4abcbf57125304d52839b0cfc13fbd0c2ae5b10c7f60374009e85ac47252bde2",
			GeneratedCodecRoute:              "go_source_instance_projection_contextual_raw_tokens",
			ReportPath:                       "target/tct-syntax-real-source-demo/go-net-lookup-plan9-source-instance-generated-codec-report-v1.json",
			GeneratedCodecReportArtifactKind: "tct_go_syntax_source_instance_generated_codec_report_v1",
			ExpectedTCTTokenCount:            21152,
			ExpectedTCTTokensSHA256:          "912a6a4456bf4588a173b58767062178c1a1897416b3925f54289ff36e6849ee",
			ExpectedMaxRawTokenID:            376,
			ExpectedVocabSize:                577,
			ExpectedPadToken:                 576,
			ExpectedPadTokenAbsent:           &padTokenAbsent,
			GoParserVersion:                  "go1.24.3",
		}},
	})
	writeTestInventory(t, inventoryPath, "File", "FuncDecl")

	report, err := buildIntakeReport(intakeOptions{
		selectionReportPath:          selectionPath,
		preferredManifestPath:        "net/lookup_plan9.go",
		existingManifestPath:         existingPath,
		metadataRoot:                 metadataRoot,
		reportIngestionManifest:      reportIngestionPath,
		rendererCodegenInventoryPath: inventoryPath,
	})
	if err != nil {
		t.Fatal(err)
	}
	if report.Status != intakeStatusReady {
		t.Fatalf("expected ready intake status, got %s with blockers %v", report.Status, blockerKinds(report.Blockers))
	}
	if !report.CurrentEvidence.GeneratedCodecReportFound {
		t.Fatal("source-instance report-ingestion record should prove generated-codec report presence")
	}
	if report.CurrentEvidence.GeneratedCodecReportArtifactKind != "tct_go_syntax_source_instance_generated_codec_report_v1" {
		t.Fatalf("unexpected report artifact kind: %+v", report.CurrentEvidence)
	}
	if report.CurrentEvidence.GeneratedCodecExpectedTCTTokenCount != 21152 {
		t.Fatalf("unexpected TCT token count: %+v", report.CurrentEvidence)
	}
	if report.CurrentEvidence.GeneratedCodecExpectedTCTTokensSHA256 != "912a6a4456bf4588a173b58767062178c1a1897416b3925f54289ff36e6849ee" {
		t.Fatalf("unexpected TCT token hash: %+v", report.CurrentEvidence)
	}
	if report.CurrentEvidence.GeneratedCodecGoParserVersion != "go1.24.3" {
		t.Fatalf("unexpected Go parser version: %+v", report.CurrentEvidence)
	}
	if !report.CurrentEvidence.GeneratedCodecSourceRoute.Found {
		t.Fatal("source-instance report record should prove selected-source route support")
	}
	if len(report.Blockers) != 0 {
		t.Fatalf("unexpected blockers for promoted source-instance report: %v", blockerKinds(report.Blockers))
	}
	if !strings.Contains(report.NextConcreteTask, "package and validate") {
		t.Fatalf("next task should move to packaging validation: %s", report.NextConcreteTask)
	}
}

func TestFindReportIngestionRecordRequiresWholeSourceCompatibleExactMatch(t *testing.T) {
	checkedSourcePath := "tests/fixtures/go_syntax_corpus/src/os/signal/signal.go"
	manifestPath := "os/signal/signal.go"
	manifest := reportIngestionManifest{
		Records: []reportIngestionRecord{
			{
				Role:                "source_span_training_expansion",
				EvidenceScope:       "source_span_generated_codec",
				ManifestPath:        manifestPath,
				SourcePath:          checkedSourcePath,
				GeneratedCodecRoute: "go_os_signal_notify_context_source_span_syntax_v0",
				ReportPath:          "target/tct-syntax-real-source-demo/go-os-signal-notify-context-source-span-generated-codec-demo.json",
			},
			{
				Role:                "source_instance_training_expansion",
				EvidenceScope:       "source_instance_generated_codec",
				ManifestPath:        manifestPath,
				SourcePath:          "tests/fixtures/go_syntax_corpus/src/os/signal/other.go",
				GeneratedCodecRoute: "go_source_instance_projection_contextual_raw_tokens",
				ReportPath:          "target/tct-syntax-real-source-demo/go-os-signal-other-source-instance-generated-codec-report-v1.json",
			},
			{
				Role:                "source_instance_training_expansion",
				EvidenceScope:       "source_instance_generated_codec",
				ManifestPath:        "os/signal/other.go",
				SourcePath:          checkedSourcePath,
				GeneratedCodecRoute: "go_source_instance_projection_contextual_raw_tokens",
				ReportPath:          "target/tct-syntax-real-source-demo/go-os-signal-source-instance-generated-codec-report-v1.json",
			},
		},
	}

	if record, found := findReportIngestionRecord(manifest, manifestPath, checkedSourcePath); found {
		t.Fatalf("source-span or partial path matches must not satisfy whole-source report support: %+v", record)
	}

	manifest.Records = append(manifest.Records, reportIngestionRecord{
		Role:                             "source_instance_training_expansion",
		EvidenceScope:                    "source_instance_generated_codec",
		ManifestPath:                     manifestPath,
		SourcePath:                       checkedSourcePath,
		GeneratedCodecRoute:              "go_source_instance_projection_contextual_raw_tokens",
		ReportPath:                       "target/tct-syntax-real-source-demo/go-os-signal-source-instance-generated-codec-report-v1.json",
		GeneratedCodecReportArtifactKind: "tct_go_syntax_source_instance_generated_codec_report_v1",
	})
	record, found := findReportIngestionRecord(manifest, manifestPath, checkedSourcePath)
	if !found {
		t.Fatal("compatible source-instance report record should be found")
	}
	if record.GeneratedCodecRoute != "go_source_instance_projection_contextual_raw_tokens" {
		t.Fatalf("unexpected selected record: %+v", record)
	}
}

func TestGeneratedCodecIntakeFallsBackToSmallestSelectedCandidate(t *testing.T) {
	candidate, selectedBy, preferredSelected, err := chooseCandidate([]goSourceCandidate{
		{ManifestPath: "larger.go", File: sourceFileRef{Bytes: 20}},
		{ManifestPath: "smaller.go", File: sourceFileRef{Bytes: 10}},
	}, "missing.go")
	if err != nil {
		t.Fatal(err)
	}
	if preferredSelected {
		t.Fatal("missing preferred candidate must not be marked selected")
	}
	if selectedBy != "smallest_selected_candidate_fallback" {
		t.Fatalf("unexpected selected_by: %s", selectedBy)
	}
	if candidate.ManifestPath != "smaller.go" {
		t.Fatalf("expected smallest fallback, got %s", candidate.ManifestPath)
	}
}

func writeTestJSON(t *testing.T, path string, value any) {
	t.Helper()
	if err := os.MkdirAll(filepath.Dir(path), 0o755); err != nil {
		t.Fatal(err)
	}
	data, err := json.Marshal(value)
	if err != nil {
		t.Fatal(err)
	}
	if err := os.WriteFile(path, data, 0o644); err != nil {
		t.Fatal(err)
	}
}

func writeTestInventory(t *testing.T, path string, tokenizableNodeKinds ...string) {
	t.Helper()
	nodes := make([]map[string]any, 0, len(tokenizableNodeKinds)+1)
	for _, nodeKind := range tokenizableNodeKinds {
		nodes = append(nodes, map[string]any{
			"name":        nodeKind,
			"tokenizable": true,
		})
	}
	nodes = append(nodes, map[string]any{
		"name":        "Package",
		"tokenizable": true,
	})
	writeTestJSON(t, path, map[string]any{
		"artifact_kind": goAstSchemaInventoryArtifactKind,
		"version":       goAstSchemaInventoryVersion,
		"go": map[string]string{
			"version": "go1.24.3",
		},
		"node_structs": nodes,
	})
}

func runtimeGOROOTForTest() string {
	return filepath.Clean("/goroot")
}
