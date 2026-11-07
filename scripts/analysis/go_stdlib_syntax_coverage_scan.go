// go_stdlib_syntax_coverage_scan enumerates and classifies the Go standard
// library source tree for the stdlib-scale coverage gate (M2 increment 1).
//
// Modes:
//
//	--enumerate --goroot <dir> [--output <json>]
//	    Walk $GOROOT/src, classify every .go file, cross-check the walked set
//	    against `go list std` GoFiles, and write a provenance summary JSON
//	    plus a per-file JSONL sibling.
//	--self-test
//	    Run the enumeration and cross-check core on an embedded miniature
//	    corpus and exit 0/1.
package main

import (
	"bufio"
	"bytes"
	"crypto/sha256"
	"encoding/hex"
	"encoding/json"
	"flag"
	"fmt"
	"go/ast"
	"go/build/constraint"
	"go/parser"
	"go/token"
	"io/fs"
	"os"
	"os/exec"
	"path/filepath"
	"regexp"
	"runtime"
	"sort"
	"strings"
	"time"
)

const (
	goStdlibCoverageArtifactKind     = "tct_go_stdlib_syntax_coverage_enumeration_v1"
	goStdlibCoverageTool             = "go_stdlib_syntax_coverage_scan"
	goStdlibCoverageVersion          = 1
	goStdlibCoverageExpectedGo       = "go1.24.3"
	goStdlibCoverageParseFailurule   = "recorded_and_excluded_from_denominator"
	goStdlibCrossCheckStatusPassed   = "passed"
	goStdlibCrossCheckStatusMissing  = "missing_files"
	goStdlibVersionCheckMatched      = "matched"
	goStdlibVersionCheckOverridden   = "mismatch_overridden"
	goStdlibPerFileRecordsSuffix     = ".files.jsonl"
	goStdlibSummaryJSONFileExtension = ".json"
)

// goStdlibFileRecord is one enumerated .go file; it is both the in-memory
// classification unit and the per-file JSONL record shape.
type goStdlibFileRecord struct {
	Path               string `json:"path"`
	SHA256             string `json:"sha256"`
	Bytes              int64  `json:"bytes"`
	IsTest             bool   `json:"is_test"`
	IsBuildConstrained bool   `json:"is_build_constrained"`
	IsCgo              bool   `json:"is_cgo"`
	IsGenerated        bool   `json:"is_generated"`
	ParseFailed        bool   `json:"parse_failed"`
	ParseError         string `json:"parse_error,omitempty"`
}

type goStdlibExcludedDirCounts struct {
	Testdata int `json:"testdata"`
	Vendor   int `json:"vendor"`
	DotDirs  int `json:"dot_dirs"`
}

type goStdlibClassificationCounts struct {
	IsTest             int `json:"is_test"`
	IsBuildConstrained int `json:"is_build_constrained"`
	IsCgo              int `json:"is_cgo"`
	IsGenerated        int `json:"is_generated"`
}

type goStdlibParseFailure struct {
	Path  string `json:"path"`
	Error string `json:"error"`
}

type goStdlibEnumeration struct {
	Records           []goStdlibFileRecord
	ExcludedDirCounts goStdlibExcludedDirCounts
	SkippedNonGoFiles int
	TotalSourceBytes  int64
}

func (e *goStdlibEnumeration) classificationCounts() goStdlibClassificationCounts {
	counts := goStdlibClassificationCounts{}
	for _, record := range e.Records {
		if record.IsTest {
			counts.IsTest++
		}
		if record.IsBuildConstrained {
			counts.IsBuildConstrained++
		}
		if record.IsCgo {
			counts.IsCgo++
		}
		if record.IsGenerated {
			counts.IsGenerated++
		}
	}
	return counts
}

func (e *goStdlibEnumeration) parseFailures() []goStdlibParseFailure {
	var failures []goStdlibParseFailure
	for _, record := range e.Records {
		if record.ParseFailed {
			failures = append(failures, goStdlibParseFailure{Path: record.Path, Error: record.ParseError})
		}
	}
	return failures
}

func (e *goStdlibEnumeration) denominator() int {
	return len(e.Records) - len(e.parseFailures())
}

type goListPackage struct {
	ImportPath string
	Dir        string
	GoFiles    []string
}

type goListCrossCheckReport struct {
	GoBin                 string   `json:"go_bin"`
	GoBinVersion          string   `json:"go_bin_version"`
	PackageCount          int      `json:"package_count"`
	VendorPackagesSkipped int      `json:"vendor_packages_skipped"`
	GoFilesCount          int      `json:"go_files_count"`
	MissingCount          int      `json:"missing_count"`
	MissingFromWalk       []string `json:"missing_from_walk"`
	Status                string   `json:"status"`
}

type goStdlibEnumerationRule struct {
	WalkRoot           string   `json:"walk_root"`
	ExcludedDirs       []string `json:"excluded_dirs"`
	IncludeTests       bool     `json:"include_tests"`
	IncludeIgnored     bool     `json:"include_ignored"`
	IncludeCgo         bool     `json:"include_cgo"`
	IncludeGenerated   bool     `json:"include_generated"`
	SizeCap            string   `json:"size_cap"`
	ParseFailurePolicy string   `json:"parse_failure_policy"`
}

type goStdlibCoverageSummary struct {
	ArtifactKind         string                       `json:"artifact_kind"`
	Version              int                          `json:"version"`
	Tool                 string                       `json:"tool"`
	ToolGoVersion        string                       `json:"tool_go_version"`
	RepoCommit           string                       `json:"repo_commit"`
	GoRoot               string                       `json:"goroot"`
	GoVersion            string                       `json:"go_version"`
	ExpectedGoVersion    string                       `json:"expected_go_version"`
	GoVersionCheck       string                       `json:"go_version_check"`
	EnumerationRule      goStdlibEnumerationRule      `json:"enumeration_rule"`
	ExcludedDirCounts    goStdlibExcludedDirCounts    `json:"excluded_dir_counts"`
	SkippedNonGoFiles    int                          `json:"skipped_non_go_files"`
	FileCount            int                          `json:"file_count"`
	ParseFailedCount     int                          `json:"parse_failed_count"`
	Denominator          int                          `json:"denominator"`
	ClassificationCounts goStdlibClassificationCounts `json:"classification_counts"`
	TotalSourceBytes     int64                        `json:"total_source_bytes"`
	ParseFailures        []goStdlibParseFailure       `json:"parse_failures"`
	GoListCrossCheck     goListCrossCheckReport       `json:"go_list_cross_check"`
	PerFileRecordsPath   string                       `json:"per_file_records_path,omitempty"`
	ElapsedSeconds       float64                      `json:"elapsed_seconds"`
}

func enumerateGoStdlibSource(goroot string) (*goStdlibEnumeration, error) {
	walkRoot := filepath.Join(goroot, "src")
	info, err := os.Stat(walkRoot)
	if err != nil {
		return nil, fmt.Errorf("stat walk root %s: %w", walkRoot, err)
	}
	if !info.IsDir() {
		return nil, fmt.Errorf("walk root %s is not a directory", walkRoot)
	}

	enumeration := &goStdlibEnumeration{}
	err = filepath.WalkDir(walkRoot, func(path string, entry fs.DirEntry, walkErr error) error {
		if walkErr != nil {
			return walkErr
		}
		if entry.IsDir() {
			if path == walkRoot {
				return nil
			}
			name := entry.Name()
			switch {
			case strings.HasPrefix(name, "."):
				enumeration.ExcludedDirCounts.DotDirs++
				return filepath.SkipDir
			case name == "vendor":
				enumeration.ExcludedDirCounts.Vendor++
				return filepath.SkipDir
			case name == "testdata":
				enumeration.ExcludedDirCounts.Testdata++
				return filepath.SkipDir
			}
			return nil
		}
		if !strings.HasSuffix(entry.Name(), ".go") {
			enumeration.SkippedNonGoFiles++
			return nil
		}
		source, err := os.ReadFile(path)
		if err != nil {
			return fmt.Errorf("read %s: %w", path, err)
		}
		relPath, err := filepath.Rel(goroot, path)
		if err != nil {
			return fmt.Errorf("relativize %s: %w", path, err)
		}
		record := classifyGoStdlibSourceFile(filepath.ToSlash(relPath), entry.Name(), path, source)
		enumeration.Records = append(enumeration.Records, record)
		enumeration.TotalSourceBytes += int64(len(source))
		return nil
	})
	if err != nil {
		return nil, fmt.Errorf("walk %s: %w", walkRoot, err)
	}

	sort.Slice(enumeration.Records, func(i, j int) bool {
		return enumeration.Records[i].Path < enumeration.Records[j].Path
	})
	return enumeration, nil
}

func classifyGoStdlibSourceFile(relPath, name, fullPath string, source []byte) goStdlibFileRecord {
	digest := sha256.Sum256(source)
	record := goStdlibFileRecord{
		Path:               relPath,
		SHA256:             hex.EncodeToString(digest[:]),
		Bytes:              int64(len(source)),
		IsTest:             strings.HasSuffix(name, "_test.go"),
		IsBuildConstrained: hasGoBuildConstraint(source),
	}
	fileSet := token.NewFileSet()
	parsed, err := parser.ParseFile(fileSet, fullPath, source, parser.ParseComments|parser.SkipObjectResolution)
	if err != nil {
		record.ParseFailed = true
		record.ParseError = err.Error()
		record.IsGenerated = hasGeneratedMarkerFallback(source)
		return record
	}
	record.IsGenerated = ast.IsGenerated(parsed)
	record.IsCgo = goStdlibFileImportsC(parsed)
	return record
}

func goStdlibFileImportsC(file *ast.File) bool {
	for _, importSpec := range file.Imports {
		if importSpec.Path != nil && importSpec.Path.Value == `"C"` {
			return true
		}
	}
	return false
}

// hasGoBuildConstraint reports whether the file header (before the first
// non-comment, non-blank line) carries a //go:build or // +build constraint,
// using go/build/constraint line classification.
func hasGoBuildConstraint(source []byte) bool {
	inBlockComment := false
	for _, line := range strings.Split(string(source), "\n") {
		trimmed := strings.TrimSpace(strings.TrimSuffix(line, "\r"))
		if inBlockComment {
			if index := strings.Index(trimmed, "*/"); index >= 0 {
				inBlockComment = false
			}
			continue
		}
		if constraint.IsGoBuild(trimmed) || constraint.IsPlusBuild(trimmed) {
			return true
		}
		if strings.HasPrefix(trimmed, "/*") {
			if !strings.Contains(trimmed[2:], "*/") {
				inBlockComment = true
			}
			continue
		}
		if trimmed == "" || strings.HasPrefix(trimmed, "//") {
			continue
		}
		// First non-comment, non-blank line: build constraints must appear
		// before this point, so stop scanning.
		return false
	}
	return false
}

var goStdlibGeneratedMarkerPattern = regexp.MustCompile(`^// Code generated .* DO NOT EDIT\.$`)

// hasGeneratedMarkerFallback scans the pre-package header for the standard
// generated-code marker. It is only used when go/parser fails and
// ast.IsGenerated is unavailable.
func hasGeneratedMarkerFallback(source []byte) bool {
	for _, line := range strings.Split(string(source), "\n") {
		line = strings.TrimSuffix(line, "\r")
		if strings.HasPrefix(strings.TrimSpace(line), "package ") {
			return false
		}
		if goStdlibGeneratedMarkerPattern.MatchString(line) {
			return true
		}
	}
	return false
}

func runGoListStd(goBin, goroot string) ([]goListPackage, error) {
	command := exec.Command(goBin, "list", "-f", "{{.ImportPath}}|{{.Dir}}|{{join .GoFiles \",\"}}", "std")
	command.Env = append(os.Environ(), "GOROOT="+goroot)
	var stdout, stderr bytes.Buffer
	command.Stdout = &stdout
	command.Stderr = &stderr
	if err := command.Run(); err != nil {
		return nil, fmt.Errorf("%s list std failed: %v: %s", goBin, err, strings.TrimSpace(stderr.String()))
	}
	return parseGoListPackages(stdout.Bytes())
}

func parseGoListPackages(output []byte) ([]goListPackage, error) {
	var packages []goListPackage
	for _, line := range strings.Split(string(output), "\n") {
		line = strings.TrimSpace(line)
		if line == "" {
			continue
		}
		parts := strings.SplitN(line, "|", 3)
		if len(parts) != 3 {
			return nil, fmt.Errorf("malformed go list line %q", line)
		}
		var goFiles []string
		for _, file := range strings.Split(parts[2], ",") {
			file = strings.TrimSpace(file)
			if file != "" {
				goFiles = append(goFiles, file)
			}
		}
		packages = append(packages, goListPackage{
			ImportPath: parts[0],
			Dir:        parts[1],
			GoFiles:    goFiles,
		})
	}
	return packages, nil
}

func crossCheckGoListPackages(packages []goListPackage, goroot string, walked map[string]bool) goListCrossCheckReport {
	report := goListCrossCheckReport{MissingFromWalk: []string{}}
	for _, listPackage := range packages {
		if listPackage.ImportPath == "vendor" ||
			strings.HasPrefix(listPackage.ImportPath, "vendor/") ||
			strings.Contains(listPackage.ImportPath, "/vendor/") {
			report.VendorPackagesSkipped++
			continue
		}
		report.PackageCount++
		for _, file := range listPackage.GoFiles {
			report.GoFilesCount++
			fullPath := filepath.Join(listPackage.Dir, file)
			relPath, err := filepath.Rel(goroot, fullPath)
			if err != nil || strings.HasPrefix(relPath, "..") {
				report.MissingFromWalk = append(report.MissingFromWalk, filepath.ToSlash(fullPath))
				continue
			}
			normalized := filepath.ToSlash(relPath)
			if !walked[normalized] {
				report.MissingFromWalk = append(report.MissingFromWalk, normalized)
			}
		}
	}
	sort.Strings(report.MissingFromWalk)
	report.MissingCount = len(report.MissingFromWalk)
	if report.MissingCount == 0 {
		report.Status = goStdlibCrossCheckStatusPassed
	} else {
		report.Status = goStdlibCrossCheckStatusMissing
	}
	return report
}

func readGoRootVersion(goroot string) (string, error) {
	versionPath := filepath.Join(goroot, "VERSION")
	data, err := os.ReadFile(versionPath)
	if err != nil {
		return "", fmt.Errorf("read %s: %w", versionPath, err)
	}
	firstLine := strings.SplitN(string(data), "\n", 2)[0]
	version := strings.TrimSpace(firstLine)
	if version == "" {
		return "", fmt.Errorf("%s has an empty first line", versionPath)
	}
	return version, nil
}

func goBinGoVersion(goBin, goroot string) (string, error) {
	command := exec.Command(goBin, "env", "GOVERSION")
	command.Env = append(os.Environ(), "GOROOT="+goroot)
	var stdout, stderr bytes.Buffer
	command.Stdout = &stdout
	command.Stderr = &stderr
	if err := command.Run(); err != nil {
		return "", fmt.Errorf("%s env GOVERSION failed: %v: %s", goBin, err, strings.TrimSpace(stderr.String()))
	}
	return strings.TrimSpace(stdout.String()), nil
}

func currentRepoCommit() string {
	output, err := exec.Command("git", "rev-parse", "HEAD").Output()
	if err != nil {
		return "unavailable"
	}
	return strings.TrimSpace(string(output))
}

func derivePerFileRecordsPath(summaryPath string) string {
	return strings.TrimSuffix(summaryPath, goStdlibSummaryJSONFileExtension) + goStdlibPerFileRecordsSuffix
}

func writePerFileRecords(path string, records []goStdlibFileRecord) error {
	if err := os.MkdirAll(filepath.Dir(path), 0o755); err != nil {
		return fmt.Errorf("create per-file records directory: %w", err)
	}
	file, err := os.Create(path)
	if err != nil {
		return fmt.Errorf("create per-file records %s: %w", path, err)
	}
	defer file.Close()
	writer := bufio.NewWriter(file)
	encoder := json.NewEncoder(writer)
	for _, record := range records {
		if err := encoder.Encode(record); err != nil {
			return fmt.Errorf("encode per-file record %s: %w", record.Path, err)
		}
	}
	if err := writer.Flush(); err != nil {
		return fmt.Errorf("flush per-file records %s: %w", path, err)
	}
	return file.Close()
}

func runEnumerateMode(goroot, output, filesOutput, goBin, expectedGoVersion string, allowVersionMismatch bool) error {
	start := time.Now()
	absoluteGoroot, err := filepath.Abs(goroot)
	if err != nil {
		return fmt.Errorf("absolutize --goroot %s: %w", goroot, err)
	}

	gorootVersion, err := readGoRootVersion(absoluteGoroot)
	if err != nil {
		return err
	}
	goBinVersion, err := goBinGoVersion(goBin, absoluteGoroot)
	if err != nil {
		return err
	}
	toolGoVersion := runtime.Version()
	var mismatches []string
	if gorootVersion != expectedGoVersion {
		mismatches = append(mismatches, fmt.Sprintf("goroot VERSION is %s", gorootVersion))
	}
	if goBinVersion != expectedGoVersion {
		mismatches = append(mismatches, fmt.Sprintf("go binary %s reports %s", goBin, goBinVersion))
	}
	if toolGoVersion != expectedGoVersion {
		mismatches = append(mismatches, fmt.Sprintf("scan tool was built with %s", toolGoVersion))
	}
	versionCheck := goStdlibVersionCheckMatched
	if len(mismatches) > 0 {
		if !allowVersionMismatch {
			return fmt.Errorf(
				"GO VERSION MISMATCH: expected %s but %s; the stdlib coverage denominator is only "+
					"authoritative against the pinned toolchain; pass --allow-go-version-mismatch to override",
				expectedGoVersion, strings.Join(mismatches, "; "))
		}
		versionCheck = goStdlibVersionCheckOverridden
	}

	enumeration, err := enumerateGoStdlibSource(absoluteGoroot)
	if err != nil {
		return err
	}
	walked := make(map[string]bool, len(enumeration.Records))
	for _, record := range enumeration.Records {
		walked[record.Path] = true
	}

	packages, err := runGoListStd(goBin, absoluteGoroot)
	if err != nil {
		return err
	}
	crossCheck := crossCheckGoListPackages(packages, absoluteGoroot, walked)
	crossCheck.GoBin = goBin
	crossCheck.GoBinVersion = goBinVersion

	parseFailures := enumeration.parseFailures()
	if parseFailures == nil {
		parseFailures = []goStdlibParseFailure{}
	}
	summary := goStdlibCoverageSummary{
		ArtifactKind:      goStdlibCoverageArtifactKind,
		Version:           goStdlibCoverageVersion,
		Tool:              goStdlibCoverageTool,
		ToolGoVersion:     toolGoVersion,
		RepoCommit:        currentRepoCommit(),
		GoRoot:            filepath.ToSlash(absoluteGoroot),
		GoVersion:         gorootVersion,
		ExpectedGoVersion: expectedGoVersion,
		GoVersionCheck:    versionCheck,
		EnumerationRule: goStdlibEnumerationRule{
			WalkRoot:           filepath.ToSlash(filepath.Join(absoluteGoroot, "src")),
			ExcludedDirs:       []string{"testdata", "vendor", "dot-dirs"},
			IncludeTests:       true,
			IncludeIgnored:     true,
			IncludeCgo:         true,
			IncludeGenerated:   true,
			SizeCap:            "none",
			ParseFailurePolicy: goStdlibCoverageParseFailurule,
		},
		ExcludedDirCounts:    enumeration.ExcludedDirCounts,
		SkippedNonGoFiles:    enumeration.SkippedNonGoFiles,
		FileCount:            len(enumeration.Records),
		ParseFailedCount:     len(parseFailures),
		Denominator:          enumeration.denominator(),
		ClassificationCounts: enumeration.classificationCounts(),
		TotalSourceBytes:     enumeration.TotalSourceBytes,
		ParseFailures:        parseFailures,
		GoListCrossCheck:     crossCheck,
	}

	recordsPath := filesOutput
	if recordsPath == "" && output != "" {
		recordsPath = derivePerFileRecordsPath(output)
	}
	if recordsPath != "" {
		if err := writePerFileRecords(recordsPath, enumeration.Records); err != nil {
			return err
		}
		summary.PerFileRecordsPath = filepath.ToSlash(recordsPath)
	}
	summary.ElapsedSeconds = time.Since(start).Seconds()

	rendered, err := json.MarshalIndent(summary, "", "  ")
	if err != nil {
		return fmt.Errorf("marshal summary: %w", err)
	}
	rendered = append(rendered, '\n')
	if output == "" {
		_, err := os.Stdout.Write(rendered)
		return err
	}
	if err := os.MkdirAll(filepath.Dir(output), 0o755); err != nil {
		return fmt.Errorf("create output directory: %w", err)
	}
	if err := os.WriteFile(output, rendered, 0o644); err != nil {
		return fmt.Errorf("write summary %s: %w", output, err)
	}
	fmt.Printf(
		"wrote %s (file_count=%d parse_failed=%d denominator=%d cross_check=%s missing=%d)\n",
		output, summary.FileCount, summary.ParseFailedCount, summary.Denominator,
		summary.GoListCrossCheck.Status, summary.GoListCrossCheck.MissingCount)
	return nil
}

var goStdlibSelfTestCorpus = map[string]string{
	"src/alpha/alpha.go": `package alpha

func Add(x int) int {
	return x + 1
}
`,
	"src/alpha/alpha_test.go": `package alpha

import "testing"

func TestAdd(t *testing.T) {
	if Add(1) != 2 {
		t.Fatal("add")
	}
}
`,
	"src/beta/cgo.go": `package beta

/*
#include <stdio.h>
*/
import "C"

func UsesC() {}
`,
	"src/beta/generated.go": `// Code generated by tct-self-test. DO NOT EDIT.

package beta

func Generated() {}
`,
	"src/beta/tagged.go": `//go:build linux

package beta

func Tagged() {}
`,
	"src/beta/broken.go": `package beta

func Broken(
`,
}

var goStdlibSelfTestExcluded = map[string]string{
	"src/alpha/testdata/skip.go": "package skip\n",
	"src/vendor/vlib/vlib.go":    "package vlib\n",
	"src/.hidden/hidden.go":      "package hidden\n",
}

func runSelfTest() error {
	tempDir, err := os.MkdirTemp("", "tct-go-stdlib-coverage-scan-")
	if err != nil {
		return err
	}
	defer os.RemoveAll(tempDir)
	writeCorpusFile := func(rel, content string) error {
		path := filepath.Join(tempDir, filepath.FromSlash(rel))
		if err := os.MkdirAll(filepath.Dir(path), 0o755); err != nil {
			return err
		}
		return os.WriteFile(path, []byte(content), 0o644)
	}
	for rel, content := range goStdlibSelfTestCorpus {
		if err := writeCorpusFile(rel, content); err != nil {
			return err
		}
	}
	for rel, content := range goStdlibSelfTestExcluded {
		if err := writeCorpusFile(rel, content); err != nil {
			return err
		}
	}
	if err := writeCorpusFile("src/alpha/README.md", "not a go file\n"); err != nil {
		return err
	}

	enumeration, err := enumerateGoStdlibSource(tempDir)
	if err != nil {
		return err
	}
	if len(enumeration.Records) != len(goStdlibSelfTestCorpus) {
		return fmt.Errorf("record count = %d, want %d", len(enumeration.Records), len(goStdlibSelfTestCorpus))
	}
	wantCounts := goStdlibExcludedDirCounts{Testdata: 1, Vendor: 1, DotDirs: 1}
	if enumeration.ExcludedDirCounts != wantCounts {
		return fmt.Errorf("excluded dir counts = %+v, want %+v", enumeration.ExcludedDirCounts, wantCounts)
	}
	if enumeration.SkippedNonGoFiles != 1 {
		return fmt.Errorf("skipped non-go files = %d, want 1", enumeration.SkippedNonGoFiles)
	}
	wantClassification := goStdlibClassificationCounts{IsTest: 1, IsBuildConstrained: 1, IsCgo: 1, IsGenerated: 1}
	if counts := enumeration.classificationCounts(); counts != wantClassification {
		return fmt.Errorf("classification counts = %+v, want %+v", counts, wantClassification)
	}
	failures := enumeration.parseFailures()
	if len(failures) != 1 || failures[0].Path != "src/beta/broken.go" || failures[0].Error == "" {
		return fmt.Errorf("unexpected parse failures: %+v", failures)
	}
	if got, want := enumeration.denominator(), len(goStdlibSelfTestCorpus)-1; got != want {
		return fmt.Errorf("denominator = %d, want %d", got, want)
	}

	walked := map[string]bool{}
	for _, record := range enumeration.Records {
		walked[record.Path] = true
	}
	packages := []goListPackage{
		{ImportPath: "alpha", Dir: filepath.Join(tempDir, "src", "alpha"), GoFiles: []string{"alpha.go"}},
		{ImportPath: "vendor/vlib", Dir: filepath.Join(tempDir, "src", "vendor", "vlib"), GoFiles: []string{"vlib.go"}},
	}
	crossCheck := crossCheckGoListPackages(packages, tempDir, walked)
	if crossCheck.Status != goStdlibCrossCheckStatusPassed {
		return fmt.Errorf("cross-check status = %q, want %q: %+v", crossCheck.Status, goStdlibCrossCheckStatusPassed, crossCheck)
	}
	if crossCheck.PackageCount != 1 || crossCheck.VendorPackagesSkipped != 1 || crossCheck.GoFilesCount != 1 {
		return fmt.Errorf("unexpected cross-check tallies: %+v", crossCheck)
	}
	missingPackages := []goListPackage{
		{ImportPath: "gamma", Dir: filepath.Join(tempDir, "src", "gamma"), GoFiles: []string{"absent.go"}},
	}
	missingCheck := crossCheckGoListPackages(missingPackages, tempDir, walked)
	if missingCheck.Status != goStdlibCrossCheckStatusMissing || missingCheck.MissingCount != 1 {
		return fmt.Errorf("missing cross-check not detected: %+v", missingCheck)
	}

	parsed, err := parseGoListPackages([]byte("fmt|/x/src/fmt|print.go,scan.go\n"))
	if err != nil {
		return err
	}
	if len(parsed) != 1 || parsed[0].ImportPath != "fmt" || len(parsed[0].GoFiles) != 2 {
		return fmt.Errorf("unexpected parsed go list packages: %+v", parsed)
	}
	if _, err := parseGoListPackages([]byte("malformed-line\n")); err == nil {
		return fmt.Errorf("malformed go list line did not error")
	}
	return nil
}

func exitf(format string, args ...any) {
	fmt.Fprintf(os.Stderr, format+"\n", args...)
	os.Exit(1)
}

func main() {
	enumerate := flag.Bool("enumerate", false, "walk and classify the GOROOT src tree")
	goroot := flag.String("goroot", "", "GOROOT to enumerate (its src subtree is walked)")
	output := flag.String("output", "", "write the summary JSON here; per-file records go to a .files.jsonl sibling (default: summary to stdout, no per-file records)")
	filesOutput := flag.String("files-output", "", "override the per-file JSONL records path")
	goBin := flag.String("go-bin", "", "go binary for the go list cross-check (default $TCT_GO_BIN, then go)")
	expectedGoVersion := flag.String("expect-go-version", goStdlibCoverageExpectedGo, "required Go version for the scanned GOROOT, the go binary, and this tool")
	allowVersionMismatch := flag.Bool("allow-go-version-mismatch", false, "record a Go version mismatch instead of failing")
	selfTest := flag.Bool("self-test", false, "run built-in checks")
	flag.Parse()

	if *selfTest {
		if err := runSelfTest(); err != nil {
			exitf("go_stdlib_syntax_coverage_scan self-test failed: %v", err)
		}
		fmt.Println("go_stdlib_syntax_coverage_scan self-test passed")
		return
	}
	if !*enumerate {
		exitf("either --enumerate or --self-test is required")
	}
	if *goroot == "" {
		exitf("--enumerate requires --goroot")
	}
	resolvedGoBin := *goBin
	if resolvedGoBin == "" {
		resolvedGoBin = os.Getenv("TCT_GO_BIN")
	}
	if resolvedGoBin == "" {
		resolvedGoBin = "go"
	}
	if err := runEnumerateMode(*goroot, *output, *filesOutput, resolvedGoBin, *expectedGoVersion, *allowVersionMismatch); err != nil {
		exitf("go_stdlib_syntax_coverage_scan: %v", err)
	}
}
