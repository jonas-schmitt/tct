package main

import (
	"encoding/json"
	"os"
	"path/filepath"
	"testing"
)

func TestGoSyntaxCorpusCheckSelectionReportIsReadyWhenGrammarGapsAreCovered(t *testing.T) {
	root := t.TempDir()
	sourcePath := writeGoSyntaxCorpusCheckSource(t, root, "sample.go", goSyntaxCorpusCheckGapSource())
	selectionPath := filepath.Join(root, "selection.json")
	writeGoSyntaxCorpusCheckJSON(t, selectionPath, selectionReport{
		ArtifactKind: selectionArtifactKind,
		Version:      1,
		Tool:         "go_syntax_corpus_select",
		Status:       "ready",
		CorpusID:     "fixture-selection",
		SourceRoot:   root,
		SelectedCandidates: []selectionCandidate{{
			ManifestPath: "sample.go",
			SourcePath:   filepath.ToSlash(sourcePath),
		}},
	})

	report, err := analyzeSelectionReport(selectionPath, goSyntaxCorpusCheckMetadataRootForTest(t))
	if err != nil {
		t.Fatalf("analyze selection report: %v", err)
	}
	if report.Status != "ready" {
		t.Fatalf("status = %s", report.Status)
	}
	if report.NextMissingGrammarGap != nil {
		t.Fatalf("unexpected next grammar gap: %#v", report.NextMissingGrammarGap)
	}
	if len(report.UnsupportedGrammarGaps) != 0 {
		t.Fatalf("unexpected grammar gaps in %#v", report.UnsupportedGrammarGaps)
	}
}

func TestGoSyntaxCorpusCheckManifestAllowsGrammarGapsWithLegacyEmptyConstructExpectation(t *testing.T) {
	root := t.TempDir()
	sourcePath := writeGoSyntaxCorpusCheckSource(t, root, "sample.go", goSyntaxCorpusCheckGapSource())
	manifestPath := writeGoSyntaxCorpusCheckManifest(t, root, sourcePath)

	report, err := analyzeCorpus(manifestPath, goSyntaxCorpusCheckMetadataRootForTest(t))
	if err != nil {
		t.Fatalf("analyze corpus: %v", err)
	}
	if report.NextMissingConstruct != nil {
		t.Fatalf("legacy construct gap should remain empty, got %#v", report.NextMissingConstruct)
	}
	if report.NextMissingGrammarGap != nil {
		t.Fatalf("unexpected next grammar gap: %#v", report.NextMissingGrammarGap)
	}
}

func writeGoSyntaxCorpusCheckSource(t *testing.T, root, rel, source string) string {
	t.Helper()
	path := filepath.Join(root, filepath.FromSlash(rel))
	if err := os.MkdirAll(filepath.Dir(path), 0o755); err != nil {
		t.Fatalf("mkdir: %v", err)
	}
	if err := os.WriteFile(path, []byte(source), 0o644); err != nil {
		t.Fatalf("write source: %v", err)
	}
	return path
}

func writeGoSyntaxCorpusCheckManifest(t *testing.T, root string, sourcePath string) string {
	t.Helper()
	sourceInfo, err := fileInfo(sourcePath, sourcePath)
	if err != nil {
		t.Fatalf("source info: %v", err)
	}
	sourceManifestPath := filepath.Join(root, "sources.txt")
	if err := os.WriteFile(sourceManifestPath, []byte(sourceManifestMarker+"\nsample.go\n"), 0o644); err != nil {
		t.Fatalf("write source manifest: %v", err)
	}
	sourceManifestInfo, err := fileInfo(sourceManifestPath, sourceManifestPath)
	if err != nil {
		t.Fatalf("source manifest info: %v", err)
	}
	licensePath := filepath.Join(root, "LICENSE")
	if err := os.WriteFile(licensePath, []byte("fixture license\n"), 0o644); err != nil {
		t.Fatalf("write license: %v", err)
	}
	entry := manifestEntry{
		ManifestPath: "sample.go",
		File:         sourceInfo,
		License:      "test-only",
		SourceURI:    "fixture://sample.go",
	}
	manifestPath := filepath.Join(root, "manifest-v1.json")
	writeGoSyntaxCorpusCheckJSON(t, manifestPath, corpusManifest{
		ArtifactKind:   corpusManifestKind,
		Version:        1,
		CorpusID:       "fixture-corpus",
		SourceRoot:     root,
		SourceManifest: sourceManifestPath,
		RouteSourceManifestShape: routeSourceInfo{
			Tool:                "route_source_manifest",
			Version:             1,
			Target:              "go-syntax",
			Marker:              "tct-go-syntax-route-corpus-v1",
			CorpusFingerprint:   corpusFingerprint([]manifestEntry{entry}),
			SourceListSHA256:    sourceManifestInfo.SHA256,
			MinimumEntryCount:   1,
			FixtureScope:        true,
			RouteDecisionMarker: "tct-go-syntax-route-corpus-v1",
		},
		Selection: selectionInfo{
			Source:               "fixture",
			SourceRevision:       "fixture",
			License:              "test-only",
			LicenseTextPath:      filepath.ToSlash(licensePath),
			SelectionNote:        "fixture",
			ExpectedNextGap:      "",
			ExpectedEntryCount:   1,
			RepresentativeStatus: "fixture",
		},
		Entries:                []manifestEntry{entry},
		NoStablePromotionClaim: noStablePromotionClaim,
	})
	return manifestPath
}

func writeGoSyntaxCorpusCheckJSON(t *testing.T, path string, value any) {
	t.Helper()
	rendered, err := json.MarshalIndent(value, "", "  ")
	if err != nil {
		t.Fatalf("marshal json: %v", err)
	}
	rendered = append(rendered, '\n')
	if err := os.WriteFile(path, rendered, 0o644); err != nil {
		t.Fatalf("write json: %v", err)
	}
}

func goSyntaxCorpusCheckGapSource() string {
	return `package sample

import "fmt" // retained import spec comment

var answer = 1

type namedAnswer int // retained type spec comment

func Spread(values []int) []int {
	return append([]int{}, values...)
}

func Pick(value any) int {
	switch typed := value.(type) {
	case int:
		return typed
	default:
		return 0
	}
}

func Pair[A, B any](a A, b B) string {
	return "pair"
}

func UsePair() string {
	return Pair[int, string](1, "x")
}
`
}

func goSyntaxCorpusCheckMetadataRootForTest(t *testing.T) string {
	t.Helper()
	cwd, err := os.Getwd()
	if err != nil {
		t.Fatalf("get cwd: %v", err)
	}
	for {
		candidate := filepath.Join(cwd, defaultMetadataRoot)
		if info, err := os.Stat(candidate); err == nil && info.IsDir() {
			return candidate
		}
		parent := filepath.Dir(cwd)
		if parent == cwd {
			t.Fatalf("could not find %s from test cwd", defaultMetadataRoot)
		}
		cwd = parent
	}
}
