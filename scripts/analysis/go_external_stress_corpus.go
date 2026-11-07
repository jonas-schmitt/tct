// go_external_stress_corpus prepares and summarizes a version-pinned Go
// external stress corpus for secondary generalization / gap probes after
// stdlib M2 closed at 100%.
//
// Modes:
//
//	--self-test
//	    Offline pin validation, path sanitization, miniature enumerate, and
//	    summarize fixtures.
//	--fetch --pins <pins.json> --corpus-root <dir>
//	    Shallow-clone each pinned module under corpus-root/modules/.
//	--enumerate --corpus-root <dir> [--pins <pins.json>] --output <json>
//	    Walk fetched modules, classify .go files (same policy as M2 stdlib
//	    enumeration: exclude testdata/vendor/dot-dirs; include tests/cgo/
//	    generated/build-constrained; parse failures excluded from
//	    denominator), and write summary JSON + .files.jsonl compatible with
//	    tct-syntax-go-tokenizer-generator --stdlib-coverage-records.
//	--summarize --coverage-report <json> --enumeration <json>
//	    [--pins <pins.json>] --output <json> [--markdown <md>]
//	    Aggregate the generator coverage report into a claim-bounded external
//	    stress report with per-module and gap-bucket breakdowns.
//
// Claim boundary: secondary generalization / gap probe only. Do not fold
// results into the frozen M5 stdlib handoff without an explicit go-m5-*-v2
// protocol.
package main

import (
	"bufio"
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
	stressPinsArtifactKind         = "tct_go_external_stress_pins_v1"
	stressEnumerationArtifactKind  = "tct_go_external_stress_enumeration_v1"
	stressReportArtifactKind       = "tct_go_external_stress_coverage_report_v1"
	stressTool                     = "go_external_stress_corpus"
	stressVersion                  = 1
	stressExpectedGo               = "go1.24.3"
	stressCoverageReportKind       = "tct_go_stdlib_syntax_tokenizer_coverage_report_v1"
	stressParseFailurePolicy       = "recorded_and_excluded_from_denominator"
	stressModulesDirName           = "modules"
	stressResolvedPinsFileName     = "pins.resolved.json"
	stressDefaultPinsPath          = "tests/fixtures/go_external_stress/pins-v1.json"
	stressClaimBoundary            = "Secondary Go generalization / gap probe over version-pinned real modules. Pipeline reuses the M2 gofmt-canonical projection, raw syntax state machine, elided stream, and render/reparse gates. Not folded into the frozen M5 stdlib handoff package; not BPE validity authority; not type correctness; not stable-backend promotion; not a paper totality claim."
	stressStreamContract           = "raw-syntax-stream-elided-v1"
	stressGrammarArtifact          = "tct-go-grammar-artifact-v1"
)

// --- pin shapes ---

type stressPinModule struct {
	ID               string   `json:"id"`
	ModulePath       string   `json:"module_path"`
	GitURL           string   `json:"git_url"`
	Ref              string   `json:"ref"`
	RefKind          string   `json:"ref_kind"`
	Role             string   `json:"role"`
	IncludePrefixes  []string `json:"include_prefixes"`
	Notes            string   `json:"notes,omitempty"`
	ResolvedCommit   string   `json:"resolved_commit,omitempty"`
	CheckoutRelPath  string   `json:"checkout_rel_path,omitempty"`
	FetchStatus      string   `json:"fetch_status,omitempty"`
	FetchElapsedSec  float64  `json:"fetch_elapsed_seconds,omitempty"`
}

type stressPinsDocument struct {
	ArtifactKind      string            `json:"artifact_kind"`
	Version           int               `json:"version"`
	Tool              string            `json:"tool"`
	ExpectedGoVersion string            `json:"expected_go_version"`
	ClaimBoundary     string            `json:"claim_boundary"`
	Modules           []stressPinModule `json:"modules"`
	RepoCommit        string            `json:"repo_commit,omitempty"`
	CorpusRoot        string            `json:"corpus_root,omitempty"`
	ResolvedAt        string            `json:"resolved_at,omitempty"`
}

// --- enumeration shapes (compatible with stdlib coverage JSONL) ---

type stressFileRecord struct {
	Path               string `json:"path"`
	SHA256             string `json:"sha256"`
	Bytes              int64  `json:"bytes"`
	IsTest             bool   `json:"is_test"`
	IsBuildConstrained bool   `json:"is_build_constrained"`
	IsCgo              bool   `json:"is_cgo"`
	IsGenerated        bool   `json:"is_generated"`
	ParseFailed        bool   `json:"parse_failed"`
	ParseError         string `json:"parse_error,omitempty"`
	ModuleID           string `json:"module_id,omitempty"`
}

type stressExcludedDirCounts struct {
	Testdata int `json:"testdata"`
	Vendor   int `json:"vendor"`
	DotDirs  int `json:"dot_dirs"`
}

type stressClassificationCounts struct {
	IsTest             int `json:"is_test"`
	IsBuildConstrained int `json:"is_build_constrained"`
	IsCgo              int `json:"is_cgo"`
	IsGenerated        int `json:"is_generated"`
}

type stressParseFailure struct {
	Path  string `json:"path"`
	Error string `json:"error"`
}

type stressModuleEnumSummary struct {
	ModuleID         string `json:"module_id"`
	ModulePath       string `json:"module_path"`
	Role             string `json:"role"`
	CheckoutRelPath  string `json:"checkout_rel_path"`
	ResolvedCommit   string `json:"resolved_commit"`
	FileCount        int    `json:"file_count"`
	ParseFailedCount int    `json:"parse_failed_count"`
	Denominator      int    `json:"denominator"`
	TotalSourceBytes int64  `json:"total_source_bytes"`
}

type stressEnumerationRule struct {
	WalkRoots          []string `json:"walk_roots"`
	ExcludedDirs       []string `json:"excluded_dirs"`
	IncludeTests       bool     `json:"include_tests"`
	IncludeCgo         bool     `json:"include_cgo"`
	IncludeGenerated   bool     `json:"include_generated"`
	ParseFailurePolicy string   `json:"parse_failure_policy"`
}

type stressEnumerationSummary struct {
	ArtifactKind         string                      `json:"artifact_kind"`
	Version              int                         `json:"version"`
	Tool                 string                      `json:"tool"`
	ToolGoVersion        string                      `json:"tool_go_version"`
	RepoCommit           string                      `json:"repo_commit"`
	ExpectedGoVersion    string                      `json:"expected_go_version"`
	CorpusRoot           string                      `json:"corpus_root"`
	PinsPath             string                      `json:"pins_path"`
	ClaimBoundary        string                      `json:"claim_boundary"`
	EnumerationRule      stressEnumerationRule       `json:"enumeration_rule"`
	ExcludedDirCounts    stressExcludedDirCounts     `json:"excluded_dir_counts"`
	SkippedNonGoFiles    int                         `json:"skipped_non_go_files"`
	FileCount            int                         `json:"file_count"`
	ParseFailedCount     int                         `json:"parse_failed_count"`
	Denominator          int                         `json:"denominator"`
	ClassificationCounts stressClassificationCounts  `json:"classification_counts"`
	TotalSourceBytes     int64                       `json:"total_source_bytes"`
	ParseFailures        []stressParseFailure        `json:"parse_failures"`
	Modules              []stressModuleEnumSummary   `json:"modules"`
	PerFileRecordsPath   string                      `json:"per_file_records_path,omitempty"`
	ElapsedSeconds       float64                     `json:"elapsed_seconds"`
}

// --- coverage summarize shapes (subset of generator report) ---

type stressCoverageGapBucket struct {
	Bucket      string   `json:"bucket"`
	Count       int      `json:"count"`
	SamplePaths []string `json:"sample_paths"`
}

type stressCoverageBlocker struct {
	Path          string `json:"path"`
	Bucket        string `json:"bucket"`
	Reason        string `json:"reason"`
	NextReadyTask string `json:"next_ready_task"`
}

type stressCoverageRecord struct {
	Path                    string  `json:"path"`
	Status                  string  `json:"status"`
	Bucket                  string  `json:"bucket"`
	ParseFailed             bool    `json:"parse_failed"`
	Attempted               bool    `json:"attempted"`
	RenderReparseEqual      bool    `json:"render_reparse_equal"`
	RawStateMachineComplete bool    `json:"raw_state_machine_complete"`
	Bytes                   int64   `json:"bytes"`
	RawTokenCount           int     `json:"raw_token_count"`
	CanonicalTokenCount     int     `json:"canonical_token_count"`
	ForcedTokenCount        int     `json:"forced_token_count"`
	Error                   *string `json:"error"`
}

type stressGeneratorCoverageReport struct {
	ArtifactKind                   string                    `json:"artifact_kind"`
	Status                         string                    `json:"status"`
	CoverageScope                  string                    `json:"coverage_scope"`
	TargetMet                      bool                      `json:"target_met"`
	InputFileCount                 int                       `json:"input_file_count"`
	InputDenominator               int                       `json:"input_denominator"`
	ParseFailedCount               int                       `json:"parse_failed_count"`
	AttemptedRecordCount           int                       `json:"attempted_record_count"`
	SelectedDenominator            int                       `json:"selected_denominator"`
	NotAttemptedCount              int                       `json:"not_attempted_count"`
	SupportedRecordCount           int                       `json:"supported_record_count"`
	BlockedRecordCount             int                       `json:"blocked_record_count"`
	RawStateMachineCompleteCount   int                       `json:"raw_state_machine_complete_count"`
	RenderReparseEqualCount        int                       `json:"render_reparse_equal_count"`
	AcceptedPercent                float64                   `json:"accepted_percent"`
	RenderReparseEqualPercent      float64                   `json:"render_reparse_equal_percent"`
	SelectedAcceptedPercent        float64                   `json:"selected_accepted_percent"`
	SelectedRenderReparseEqualPct  float64                   `json:"selected_render_reparse_equal_percent"`
	TotalRawTokenCount             int64                     `json:"total_raw_token_count"`
	TotalCanonicalTokenCount       int64                     `json:"total_canonical_token_count"`
	TotalForcedTokenCount          int64                     `json:"total_forced_token_count"`
	TokenLayoutVocabSize           int                       `json:"token_layout_vocab_size"`
	TokenLayoutPadToken            int                       `json:"token_layout_pad_token"`
	GrammarArtifactID              string                    `json:"grammar_artifact_id"`
	GrammarArtifactContentSHA256   string                    `json:"grammar_artifact_content_sha256"`
	StreamContract                 string                    `json:"stream_contract"`
	GapBuckets                     []stressCoverageGapBucket `json:"gap_buckets"`
	FirstBlocker                   *stressCoverageBlocker    `json:"first_blocker"`
	Records                        []stressCoverageRecord    `json:"records"`
	FileRecordsSHA256              string                    `json:"file_records_sha256"`
}

type stressModuleCoverageSummary struct {
	ModuleID                   string  `json:"module_id"`
	Role                       string  `json:"role"`
	RecordCount                int     `json:"record_count"`
	ParseFailedCount           int     `json:"parse_failed_count"`
	Denominator                int     `json:"denominator"`
	AttemptedCount             int     `json:"attempted_count"`
	SupportedCount             int     `json:"supported_count"`
	BlockedCount               int     `json:"blocked_count"`
	RenderReparseEqualCount    int     `json:"render_reparse_equal_count"`
	AcceptedPercent            float64 `json:"accepted_percent"`
	RenderReparseEqualPercent  float64 `json:"render_reparse_equal_percent"`
	TotalSourceBytes           int64   `json:"total_source_bytes"`
	TotalRawTokenCount         int64   `json:"total_raw_token_count"`
	TotalCanonicalTokenCount   int64   `json:"total_canonical_token_count"`
	TotalForcedTokenCount      int64   `json:"total_forced_token_count"`
}

type stressCuratedReport struct {
	ArtifactKind                 string                       `json:"artifact_kind"`
	Version                      int                          `json:"version"`
	Tool                         string                       `json:"tool"`
	RepoCommit                   string                       `json:"repo_commit"`
	ExpectedGoVersion            string                       `json:"expected_go_version"`
	ClaimBoundary                string                       `json:"claim_boundary"`
	StreamContract               string                       `json:"stream_contract"`
	GrammarArtifactID            string                       `json:"grammar_artifact_id"`
	GrammarArtifactContentSHA256 string                       `json:"grammar_artifact_content_sha256"`
	TokenLayoutVocabSize         int                          `json:"token_layout_vocab_size"`
	TokenLayoutPadToken          int                          `json:"token_layout_pad_token"`
	PinsPath                     string                       `json:"pins_path"`
	EnumerationPath              string                       `json:"enumeration_path"`
	CoverageReportPath           string                       `json:"coverage_report_path"`
	CoverageReportKind           string                       `json:"coverage_report_kind"`
	GeneratorStatus              string                       `json:"generator_status"`
	CoverageScope                string                       `json:"coverage_scope"`
	FullDenominator              bool                         `json:"full_denominator"`
	TargetMet                    bool                         `json:"target_met"`
	InputFileCount               int                          `json:"input_file_count"`
	InputDenominator             int                          `json:"input_denominator"`
	ParseFailedCount             int                          `json:"parse_failed_count"`
	AttemptedRecordCount         int                          `json:"attempted_record_count"`
	SupportedRecordCount         int                          `json:"supported_record_count"`
	BlockedRecordCount           int                          `json:"blocked_record_count"`
	RenderReparseEqualCount      int                          `json:"render_reparse_equal_count"`
	AcceptedPercent              float64                      `json:"accepted_percent"`
	RenderReparseEqualPercent    float64                      `json:"render_reparse_equal_percent"`
	TotalRawTokenCount           int64                        `json:"total_raw_token_count"`
	TotalCanonicalTokenCount     int64                        `json:"total_canonical_token_count"`
	TotalForcedTokenCount        int64                        `json:"total_forced_token_count"`
	FileRecordsSHA256            string                       `json:"file_records_sha256"`
	Modules                      []stressModuleCoverageSummary `json:"modules"`
	GapBuckets                   []stressCoverageGapBucket     `json:"gap_buckets"`
	FirstBlocker                 *stressCoverageBlocker        `json:"first_blocker"`
	NextReadyTask                string                       `json:"next_ready_task"`
	ValidationCommands           []string                     `json:"validation_commands"`
	NoStableClaims               []string                     `json:"no_stable_claims"`
	Pins                         []stressPinModule             `json:"pins"`
}

// --- helpers ---

func exitf(format string, args ...any) {
	fmt.Fprintf(os.Stderr, format+"\n", args...)
	os.Exit(1)
}

func currentRepoCommit() string {
	output, err := exec.Command("git", "rev-parse", "HEAD").Output()
	if err != nil {
		return "unavailable"
	}
	return strings.TrimSpace(string(output))
}

func sanitizeModuleDirName(id string) string {
	replacer := strings.NewReplacer(
		"/", "__",
		"@", "__",
		":", "_",
		" ", "_",
	)
	name := replacer.Replace(id)
	name = regexp.MustCompile(`[^A-Za-z0-9._-]+`).ReplaceAllString(name, "_")
	if name == "" {
		return "module"
	}
	return name
}

func loadPins(path string) (*stressPinsDocument, error) {
	data, err := os.ReadFile(path)
	if err != nil {
		return nil, fmt.Errorf("read pins %s: %w", path, err)
	}
	var pins stressPinsDocument
	if err := json.Unmarshal(data, &pins); err != nil {
		return nil, fmt.Errorf("decode pins %s: %w", path, err)
	}
	if err := validatePins(&pins); err != nil {
		return nil, err
	}
	return &pins, nil
}

func validatePins(pins *stressPinsDocument) error {
	if pins.ArtifactKind != stressPinsArtifactKind {
		return fmt.Errorf("pins artifact_kind = %q, want %q", pins.ArtifactKind, stressPinsArtifactKind)
	}
	if pins.Version != stressVersion {
		return fmt.Errorf("pins version = %d, want %d", pins.Version, stressVersion)
	}
	if len(pins.Modules) == 0 {
		return fmt.Errorf("pins document has no modules")
	}
	seen := map[string]bool{}
	for i, mod := range pins.Modules {
		if strings.TrimSpace(mod.ID) == "" {
			return fmt.Errorf("modules[%d] missing id", i)
		}
		if seen[mod.ID] {
			return fmt.Errorf("duplicate module id %q", mod.ID)
		}
		seen[mod.ID] = true
		if strings.TrimSpace(mod.ModulePath) == "" {
			return fmt.Errorf("module %s missing module_path", mod.ID)
		}
		if strings.TrimSpace(mod.GitURL) == "" {
			return fmt.Errorf("module %s missing git_url", mod.ID)
		}
		if strings.TrimSpace(mod.Ref) == "" {
			return fmt.Errorf("module %s missing ref", mod.ID)
		}
		if mod.RefKind != "tag" && mod.RefKind != "commit" {
			return fmt.Errorf("module %s ref_kind = %q, want tag|commit", mod.ID, mod.RefKind)
		}
		if strings.TrimSpace(mod.Role) == "" {
			return fmt.Errorf("module %s missing role", mod.ID)
		}
	}
	return nil
}

func writeJSON(path string, value any) error {
	if err := os.MkdirAll(filepath.Dir(path), 0o755); err != nil {
		return fmt.Errorf("create directory for %s: %w", path, err)
	}
	rendered, err := json.MarshalIndent(value, "", "  ")
	if err != nil {
		return fmt.Errorf("marshal %s: %w", path, err)
	}
	rendered = append(rendered, '\n')
	if err := os.WriteFile(path, rendered, 0o644); err != nil {
		return fmt.Errorf("write %s: %w", path, err)
	}
	return nil
}

func writeJSONL(path string, records []stressFileRecord) error {
	if err := os.MkdirAll(filepath.Dir(path), 0o755); err != nil {
		return fmt.Errorf("create directory for %s: %w", path, err)
	}
	file, err := os.Create(path)
	if err != nil {
		return fmt.Errorf("create %s: %w", path, err)
	}
	defer file.Close()
	writer := bufio.NewWriter(file)
	encoder := json.NewEncoder(writer)
	for _, record := range records {
		if err := encoder.Encode(record); err != nil {
			return fmt.Errorf("encode record %s: %w", record.Path, err)
		}
	}
	if err := writer.Flush(); err != nil {
		return fmt.Errorf("flush %s: %w", path, err)
	}
	return file.Close()
}

func derivePerFileRecordsPath(summaryPath string) string {
	if strings.HasSuffix(summaryPath, ".json") {
		return strings.TrimSuffix(summaryPath, ".json") + ".files.jsonl"
	}
	return summaryPath + ".files.jsonl"
}

func moduleCheckoutRelPath(mod stressPinModule) string {
	if mod.CheckoutRelPath != "" {
		return mod.CheckoutRelPath
	}
	return filepath.ToSlash(filepath.Join(stressModulesDirName, sanitizeModuleDirName(mod.ID)))
}

func resolveGitCommit(checkout string) (string, error) {
	cmd := exec.Command("git", "-C", checkout, "rev-parse", "HEAD")
	out, err := cmd.Output()
	if err != nil {
		return "", fmt.Errorf("git rev-parse in %s: %w", checkout, err)
	}
	return strings.TrimSpace(string(out)), nil
}

func fetchModule(mod *stressPinModule, corpusRoot string) error {
	rel := moduleCheckoutRelPath(*mod)
	mod.CheckoutRelPath = rel
	dest := filepath.Join(corpusRoot, filepath.FromSlash(rel))
	start := time.Now()

	if info, err := os.Stat(filepath.Join(dest, ".git")); err == nil && info.IsDir() {
		commit, err := resolveGitCommit(dest)
		if err == nil {
			// If tag checkout, verify ref still matches when possible.
			mod.ResolvedCommit = commit
			mod.FetchStatus = "cache_hit"
			mod.FetchElapsedSec = time.Since(start).Seconds()
			return nil
		}
	}

	if err := os.RemoveAll(dest); err != nil {
		return fmt.Errorf("remove stale checkout %s: %w", dest, err)
	}
	if err := os.MkdirAll(filepath.Dir(dest), 0o755); err != nil {
		return fmt.Errorf("mkdir for %s: %w", dest, err)
	}

	args := []string{"clone", "--depth", "1"}
	if mod.RefKind == "tag" {
		args = append(args, "--branch", mod.Ref)
	}
	args = append(args, mod.GitURL, dest)
	cmd := exec.Command("git", args...)
	var stderr strings.Builder
	cmd.Stderr = &stderr
	if err := cmd.Run(); err != nil {
		return fmt.Errorf("git clone %s@%s: %v: %s", mod.GitURL, mod.Ref, err, strings.TrimSpace(stderr.String()))
	}
	if mod.RefKind == "commit" {
		checkout := exec.Command("git", "-C", dest, "checkout", "--detach", mod.Ref)
		var checkoutErr strings.Builder
		checkout.Stderr = &checkoutErr
		if err := checkout.Run(); err != nil {
			return fmt.Errorf("git checkout %s: %v: %s", mod.Ref, err, strings.TrimSpace(checkoutErr.String()))
		}
	}
	commit, err := resolveGitCommit(dest)
	if err != nil {
		return err
	}
	mod.ResolvedCommit = commit
	mod.FetchStatus = "cloned"
	mod.FetchElapsedSec = time.Since(start).Seconds()
	return nil
}

func runFetchMode(pinsPath, corpusRoot string) error {
	pins, err := loadPins(pinsPath)
	if err != nil {
		return err
	}
	absoluteRoot, err := filepath.Abs(corpusRoot)
	if err != nil {
		return fmt.Errorf("absolutize corpus root: %w", err)
	}
	if err := os.MkdirAll(absoluteRoot, 0o755); err != nil {
		return fmt.Errorf("create corpus root: %w", err)
	}

	for i := range pins.Modules {
		fmt.Printf("fetching %s ...\n", pins.Modules[i].ID)
		if err := fetchModule(&pins.Modules[i], absoluteRoot); err != nil {
			return err
		}
		fmt.Printf("  %s -> %s (%s)\n", pins.Modules[i].ID, pins.Modules[i].CheckoutRelPath, pins.Modules[i].FetchStatus)
	}

	pins.RepoCommit = currentRepoCommit()
	pins.CorpusRoot = filepath.ToSlash(absoluteRoot)
	pins.ResolvedAt = time.Now().UTC().Format(time.RFC3339)
	resolvedPath := filepath.Join(absoluteRoot, stressResolvedPinsFileName)
	if err := writeJSON(resolvedPath, pins); err != nil {
		return err
	}
	fmt.Printf("wrote %s (%d modules)\n", resolvedPath, len(pins.Modules))
	return nil
}

var stressGeneratedMarkerPattern = regexp.MustCompile(`^// Code generated .* DO NOT EDIT\.$`)

func hasGeneratedMarkerFallback(source []byte) bool {
	for _, line := range strings.Split(string(source), "\n") {
		line = strings.TrimSuffix(line, "\r")
		if strings.HasPrefix(strings.TrimSpace(line), "package ") {
			return false
		}
		if stressGeneratedMarkerPattern.MatchString(line) {
			return true
		}
	}
	return false
}

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
		return false
	}
	return false
}

func fileImportsC(file *ast.File) bool {
	for _, importSpec := range file.Imports {
		if importSpec.Path != nil && importSpec.Path.Value == `"C"` {
			return true
		}
	}
	return false
}

func classifyStressSourceFile(relPath, name, fullPath string, source []byte, moduleID string) stressFileRecord {
	digest := sha256.Sum256(source)
	record := stressFileRecord{
		Path:               relPath,
		SHA256:             hex.EncodeToString(digest[:]),
		Bytes:              int64(len(source)),
		IsTest:             strings.HasSuffix(name, "_test.go"),
		IsBuildConstrained: hasGoBuildConstraint(source),
		ModuleID:           moduleID,
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
	record.IsCgo = fileImportsC(parsed)
	return record
}

func pathHasPrefix(rel string, prefixes []string) bool {
	if len(prefixes) == 0 {
		return true
	}
	for _, prefix := range prefixes {
		prefix = strings.Trim(strings.TrimSpace(prefix), "/")
		if prefix == "" || rel == prefix || strings.HasPrefix(rel, prefix+"/") {
			return true
		}
	}
	return false
}

type stressWalkResult struct {
	Records           []stressFileRecord
	ExcludedDirCounts stressExcludedDirCounts
	SkippedNonGoFiles int
	TotalSourceBytes  int64
}

func enumerateModuleTree(corpusRoot, checkoutRel, moduleID string, includePrefixes []string) (*stressWalkResult, error) {
	walkRoot := filepath.Join(corpusRoot, filepath.FromSlash(checkoutRel))
	info, err := os.Stat(walkRoot)
	if err != nil {
		return nil, fmt.Errorf("stat module checkout %s: %w", walkRoot, err)
	}
	if !info.IsDir() {
		return nil, fmt.Errorf("module checkout %s is not a directory", walkRoot)
	}

	result := &stressWalkResult{}
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
				result.ExcludedDirCounts.DotDirs++
				return filepath.SkipDir
			case name == "vendor":
				result.ExcludedDirCounts.Vendor++
				return filepath.SkipDir
			case name == "testdata":
				result.ExcludedDirCounts.Testdata++
				return filepath.SkipDir
			}
			return nil
		}
		if !strings.HasSuffix(entry.Name(), ".go") {
			result.SkippedNonGoFiles++
			return nil
		}
		relFromModule, err := filepath.Rel(walkRoot, path)
		if err != nil {
			return fmt.Errorf("relativize %s: %w", path, err)
		}
		relFromModuleSlash := filepath.ToSlash(relFromModule)
		if !pathHasPrefix(relFromModuleSlash, includePrefixes) {
			return nil
		}
		source, err := os.ReadFile(path)
		if err != nil {
			return fmt.Errorf("read %s: %w", path, err)
		}
		relFromCorpus, err := filepath.Rel(corpusRoot, path)
		if err != nil {
			return fmt.Errorf("relativize corpus path %s: %w", path, err)
		}
		record := classifyStressSourceFile(filepath.ToSlash(relFromCorpus), entry.Name(), path, source, moduleID)
		result.Records = append(result.Records, record)
		result.TotalSourceBytes += int64(len(source))
		return nil
	})
	if err != nil {
		return nil, fmt.Errorf("walk %s: %w", walkRoot, err)
	}
	return result, nil
}

func classificationCounts(records []stressFileRecord) stressClassificationCounts {
	counts := stressClassificationCounts{}
	for _, record := range records {
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

func parseFailures(records []stressFileRecord) []stressParseFailure {
	var failures []stressParseFailure
	for _, record := range records {
		if record.ParseFailed {
			failures = append(failures, stressParseFailure{Path: record.Path, Error: record.ParseError})
		}
	}
	return failures
}

func runEnumerateMode(pinsPath, corpusRoot, output, filesOutput string) error {
	start := time.Now()
	absoluteRoot, err := filepath.Abs(corpusRoot)
	if err != nil {
		return fmt.Errorf("absolutize corpus root: %w", err)
	}

	resolvedPinsPath := filepath.Join(absoluteRoot, stressResolvedPinsFileName)
	var pins *stressPinsDocument
	if _, err := os.Stat(resolvedPinsPath); err == nil {
		pins, err = loadPins(resolvedPinsPath)
		if err != nil {
			return fmt.Errorf("load resolved pins: %w", err)
		}
	} else {
		pins, err = loadPins(pinsPath)
		if err != nil {
			return err
		}
		// Ensure checkout rel paths are populated even without fetch stamp.
		for i := range pins.Modules {
			pins.Modules[i].CheckoutRelPath = moduleCheckoutRelPath(pins.Modules[i])
		}
	}

	var allRecords []stressFileRecord
	var excluded stressExcludedDirCounts
	skippedNonGo := 0
	var totalBytes int64
	var moduleSummaries []stressModuleEnumSummary
	walkRoots := []string{}

	for _, mod := range pins.Modules {
		rel := moduleCheckoutRelPath(mod)
		walkRoots = append(walkRoots, filepath.ToSlash(filepath.Join(absoluteRoot, filepath.FromSlash(rel))))
		result, err := enumerateModuleTree(absoluteRoot, rel, mod.ID, mod.IncludePrefixes)
		if err != nil {
			return fmt.Errorf("enumerate %s: %w", mod.ID, err)
		}
		excluded.Testdata += result.ExcludedDirCounts.Testdata
		excluded.Vendor += result.ExcludedDirCounts.Vendor
		excluded.DotDirs += result.ExcludedDirCounts.DotDirs
		skippedNonGo += result.SkippedNonGoFiles
		totalBytes += result.TotalSourceBytes
		allRecords = append(allRecords, result.Records...)

		parseFailed := 0
		for _, record := range result.Records {
			if record.ParseFailed {
				parseFailed++
			}
		}
		moduleSummaries = append(moduleSummaries, stressModuleEnumSummary{
			ModuleID:         mod.ID,
			ModulePath:       mod.ModulePath,
			Role:             mod.Role,
			CheckoutRelPath:  rel,
			ResolvedCommit:   mod.ResolvedCommit,
			FileCount:        len(result.Records),
			ParseFailedCount: parseFailed,
			Denominator:      len(result.Records) - parseFailed,
			TotalSourceBytes: result.TotalSourceBytes,
		})
	}

	sort.Slice(allRecords, func(i, j int) bool {
		return allRecords[i].Path < allRecords[j].Path
	})
	failures := parseFailures(allRecords)
	if failures == nil {
		failures = []stressParseFailure{}
	}

	recordsPath := filesOutput
	if recordsPath == "" && output != "" {
		recordsPath = derivePerFileRecordsPath(output)
	}
	if recordsPath != "" {
		if err := writeJSONL(recordsPath, allRecords); err != nil {
			return err
		}
	}

	summary := stressEnumerationSummary{
		ArtifactKind:      stressEnumerationArtifactKind,
		Version:           stressVersion,
		Tool:              stressTool,
		ToolGoVersion:     runtime.Version(),
		RepoCommit:        currentRepoCommit(),
		ExpectedGoVersion: stressExpectedGo,
		CorpusRoot:        filepath.ToSlash(absoluteRoot),
		PinsPath:          filepath.ToSlash(pinsPath),
		ClaimBoundary:     stressClaimBoundary,
		EnumerationRule: stressEnumerationRule{
			WalkRoots:          walkRoots,
			ExcludedDirs:       []string{"testdata", "vendor", "dot-dirs"},
			IncludeTests:       true,
			IncludeCgo:         true,
			IncludeGenerated:   true,
			ParseFailurePolicy: stressParseFailurePolicy,
		},
		ExcludedDirCounts:    excluded,
		SkippedNonGoFiles:    skippedNonGo,
		FileCount:            len(allRecords),
		ParseFailedCount:     len(failures),
		Denominator:          len(allRecords) - len(failures),
		ClassificationCounts: classificationCounts(allRecords),
		TotalSourceBytes:     totalBytes,
		ParseFailures:        failures,
		Modules:              moduleSummaries,
		PerFileRecordsPath:   filepath.ToSlash(recordsPath),
		ElapsedSeconds:       time.Since(start).Seconds(),
	}
	if output == "" {
		rendered, err := json.MarshalIndent(summary, "", "  ")
		if err != nil {
			return err
		}
		_, err = os.Stdout.Write(append(rendered, '\n'))
		return err
	}
	if err := writeJSON(output, summary); err != nil {
		return err
	}
	fmt.Printf(
		"wrote %s (file_count=%d parse_failed=%d denominator=%d modules=%d)\n",
		output, summary.FileCount, summary.ParseFailedCount, summary.Denominator, len(summary.Modules),
	)
	if recordsPath != "" {
		fmt.Printf("wrote %s\n", recordsPath)
	}
	return nil
}

func coveragePercent(numerator, denominator int) float64 {
	if denominator == 0 {
		return 0
	}
	return 100.0 * float64(numerator) / float64(denominator)
}

func moduleIDForPath(path string, pins []stressPinModule) string {
	for _, mod := range pins {
		rel := moduleCheckoutRelPath(mod)
		if path == rel || strings.HasPrefix(path, rel+"/") {
			return mod.ID
		}
	}
	// Fall back to first modules/ segment.
	parts := strings.Split(path, "/")
	if len(parts) >= 2 && parts[0] == stressModulesDirName {
		return parts[1]
	}
	return "unknown"
}

func roleForModule(moduleID string, pins []stressPinModule) string {
	for _, mod := range pins {
		if mod.ID == moduleID || sanitizeModuleDirName(mod.ID) == moduleID {
			return mod.Role
		}
	}
	return "unknown"
}

func loadCoverageReport(path string) (*stressGeneratorCoverageReport, error) {
	data, err := os.ReadFile(path)
	if err != nil {
		return nil, fmt.Errorf("read coverage report %s: %w", path, err)
	}
	var report stressGeneratorCoverageReport
	if err := json.Unmarshal(data, &report); err != nil {
		return nil, fmt.Errorf("decode coverage report %s: %w", path, err)
	}
	if report.ArtifactKind != stressCoverageReportKind {
		return nil, fmt.Errorf("coverage artifact_kind = %q, want %q", report.ArtifactKind, stressCoverageReportKind)
	}
	return &report, nil
}

func loadEnumerationSummary(path string) (*stressEnumerationSummary, error) {
	data, err := os.ReadFile(path)
	if err != nil {
		return nil, fmt.Errorf("read enumeration %s: %w", path, err)
	}
	var summary stressEnumerationSummary
	if err := json.Unmarshal(data, &summary); err != nil {
		return nil, fmt.Errorf("decode enumeration %s: %w", path, err)
	}
	if summary.ArtifactKind != stressEnumerationArtifactKind {
		return nil, fmt.Errorf("enumeration artifact_kind = %q, want %q", summary.ArtifactKind, stressEnumerationArtifactKind)
	}
	return &summary, nil
}

func nextReadyTaskForGaps(fullDenominator bool, blocked int, first *stressCoverageBlocker) string {
	if !fullDenominator {
		return "re-run external stress coverage over the full enumerated denominator without --stdlib-coverage-limit"
	}
	if blocked == 0 {
		return "optional: add another bounded production-tree pin or promote I2 pause-token protocol freeze; do not fold into frozen M5 without go-m5-*-v2"
	}
	if first != nil && first.Bucket != "" {
		return fmt.Sprintf("fix reusable generator-core blocker class %q (first path %s); use batch intake / core fixes, not one-off source-file intake", first.Bucket, first.Path)
	}
	return "iterate on the first bucketed external-stress coverage gap with a reusable generator-core fix"
}

func runSummarizeMode(pinsPath, enumerationPath, coveragePath, output, markdown string) error {
	pins, err := loadPins(pinsPath)
	if err != nil {
		// Prefer resolved pins next to coverage if available.
		return err
	}
	enumeration, err := loadEnumerationSummary(enumerationPath)
	if err != nil {
		return err
	}
	coverage, err := loadCoverageReport(coveragePath)
	if err != nil {
		return err
	}

	// Prefer resolved pins when present under corpus root.
	if enumeration.CorpusRoot != "" {
		resolved := filepath.Join(enumeration.CorpusRoot, stressResolvedPinsFileName)
		if resolvedPins, err := loadPins(resolved); err == nil {
			pins = resolvedPins
		}
	}

	moduleStats := map[string]*stressModuleCoverageSummary{}
	for _, mod := range pins.Modules {
		moduleStats[mod.ID] = &stressModuleCoverageSummary{
			ModuleID: mod.ID,
			Role:     mod.Role,
		}
	}

	for _, record := range coverage.Records {
		moduleID := moduleIDForPath(record.Path, pins.Modules)
		stats := moduleStats[moduleID]
		if stats == nil {
			stats = &stressModuleCoverageSummary{
				ModuleID: moduleID,
				Role:     roleForModule(moduleID, pins.Modules),
			}
			moduleStats[moduleID] = stats
		}
		stats.RecordCount++
		stats.TotalSourceBytes += record.Bytes
		if record.ParseFailed {
			stats.ParseFailedCount++
			continue
		}
		stats.Denominator++
		if record.Attempted {
			stats.AttemptedCount++
		}
		if record.Status == "passed" {
			stats.SupportedCount++
		}
		if record.Status == "blocked" {
			stats.BlockedCount++
		}
		if record.RenderReparseEqual {
			stats.RenderReparseEqualCount++
		}
		stats.TotalRawTokenCount += int64(record.RawTokenCount)
		stats.TotalCanonicalTokenCount += int64(record.CanonicalTokenCount)
		stats.TotalForcedTokenCount += int64(record.ForcedTokenCount)
	}

	modules := make([]stressModuleCoverageSummary, 0, len(moduleStats))
	for _, mod := range pins.Modules {
		if stats, ok := moduleStats[mod.ID]; ok {
			stats.AcceptedPercent = coveragePercent(stats.SupportedCount, stats.Denominator)
			stats.RenderReparseEqualPercent = coveragePercent(stats.RenderReparseEqualCount, stats.Denominator)
			modules = append(modules, *stats)
			delete(moduleStats, mod.ID)
		}
	}
	// Any unexpected modules last, sorted.
	var extraIDs []string
	for id := range moduleStats {
		extraIDs = append(extraIDs, id)
	}
	sort.Strings(extraIDs)
	for _, id := range extraIDs {
		stats := moduleStats[id]
		stats.AcceptedPercent = coveragePercent(stats.SupportedCount, stats.Denominator)
		stats.RenderReparseEqualPercent = coveragePercent(stats.RenderReparseEqualCount, stats.Denominator)
		modules = append(modules, *stats)
	}

	fullDenominator := coverage.CoverageScope == "full_denominator" && coverage.NotAttemptedCount == 0
	report := stressCuratedReport{
		ArtifactKind:                 stressReportArtifactKind,
		Version:                      stressVersion,
		Tool:                         stressTool,
		RepoCommit:                   currentRepoCommit(),
		ExpectedGoVersion:            stressExpectedGo,
		ClaimBoundary:                stressClaimBoundary,
		StreamContract:               firstNonEmpty(coverage.StreamContract, stressStreamContract),
		GrammarArtifactID:            firstNonEmpty(coverage.GrammarArtifactID, stressGrammarArtifact),
		GrammarArtifactContentSHA256: coverage.GrammarArtifactContentSHA256,
		TokenLayoutVocabSize:         coverage.TokenLayoutVocabSize,
		TokenLayoutPadToken:          coverage.TokenLayoutPadToken,
		PinsPath:                     filepath.ToSlash(pinsPath),
		EnumerationPath:              filepath.ToSlash(enumerationPath),
		CoverageReportPath:           filepath.ToSlash(coveragePath),
		CoverageReportKind:           coverage.ArtifactKind,
		GeneratorStatus:              coverage.Status,
		CoverageScope:                coverage.CoverageScope,
		FullDenominator:              fullDenominator,
		TargetMet:                    fullDenominator && coverage.RenderReparseEqualPercent >= 95.0,
		InputFileCount:               coverage.InputFileCount,
		InputDenominator:             coverage.InputDenominator,
		ParseFailedCount:             coverage.ParseFailedCount,
		AttemptedRecordCount:         coverage.AttemptedRecordCount,
		SupportedRecordCount:         coverage.SupportedRecordCount,
		BlockedRecordCount:           coverage.BlockedRecordCount,
		RenderReparseEqualCount:      coverage.RenderReparseEqualCount,
		AcceptedPercent:              coverage.AcceptedPercent,
		RenderReparseEqualPercent:    coverage.RenderReparseEqualPercent,
		TotalRawTokenCount:           coverage.TotalRawTokenCount,
		TotalCanonicalTokenCount:     coverage.TotalCanonicalTokenCount,
		TotalForcedTokenCount:        coverage.TotalForcedTokenCount,
		FileRecordsSHA256:            coverage.FileRecordsSHA256,
		Modules:                      modules,
		GapBuckets:                   coverage.GapBuckets,
		FirstBlocker:                 coverage.FirstBlocker,
		NextReadyTask:                nextReadyTaskForGaps(fullDenominator, coverage.BlockedRecordCount, coverage.FirstBlocker),
		ValidationCommands: []string{
			"go run scripts/analysis/go_external_stress_corpus.go --self-test",
			"go test scripts/analysis/go_external_stress_corpus.go scripts/analysis/go_external_stress_corpus_test.go",
			fmt.Sprintf("go run scripts/analysis/go_external_stress_corpus.go --fetch --pins %s --corpus-root target/go-external-stress-corpus/v1", stressDefaultPinsPath),
			fmt.Sprintf("go run scripts/analysis/go_external_stress_corpus.go --enumerate --pins %s --corpus-root target/go-external-stress-corpus/v1 --output target/go-external-stress-corpus/v1/enumeration.json", stressDefaultPinsPath),
			"cargo run --release --features experimental --bin tct-syntax-go-tokenizer-generator -- --stdlib-coverage-records target/go-external-stress-corpus/v1/enumeration.files.jsonl --stdlib-coverage-goroot target/go-external-stress-corpus/v1 --stdlib-coverage-output-dir target/go-external-stress-corpus/v1/coverage-work --output target/go-external-stress-corpus/v1/coverage-report-v1.json",
			"cargo run --release --features experimental --bin tct-syntax-go-tokenizer-generator -- --validate-report target/go-external-stress-corpus/v1/coverage-report-v1.json",
			fmt.Sprintf("go run scripts/analysis/go_external_stress_corpus.go --summarize --pins %s --enumeration target/go-external-stress-corpus/v1/enumeration.json --coverage-report target/go-external-stress-corpus/v1/coverage-report-v1.json --output reports/go-external-stress-corpus-v1.json --markdown reports/go-external-stress-corpus-v1.md", stressDefaultPinsPath),
		},
		NoStableClaims: []string{
			"not_folded_into_frozen_m5_stdlib_handoff",
			"not_bpe_validity_authority",
			"not_type_correctness",
			"not_stable_backend_promotion",
			"not_paper_totality_claim",
			"secondary_generalization_gap_probe_only",
		},
		Pins: pins.Modules,
	}

	if output != "" {
		if err := writeJSON(output, report); err != nil {
			return err
		}
		fmt.Printf("wrote %s\n", output)
	} else {
		rendered, err := json.MarshalIndent(report, "", "  ")
		if err != nil {
			return err
		}
		if _, err := os.Stdout.Write(append(rendered, '\n')); err != nil {
			return err
		}
	}
	if markdown != "" {
		if err := os.WriteFile(markdown, []byte(renderStressMarkdown(&report)), 0o644); err != nil {
			return fmt.Errorf("write markdown %s: %w", markdown, err)
		}
		fmt.Printf("wrote %s\n", markdown)
	}
	return nil
}

func firstNonEmpty(values ...string) string {
	for _, value := range values {
		if strings.TrimSpace(value) != "" {
			return value
		}
	}
	return ""
}

func renderStressMarkdown(report *stressCuratedReport) string {
	var b strings.Builder
	b.WriteString("# Go External Stress Corpus (pinned modules)\n\n")
	b.WriteString(fmt.Sprintf("Date: %s  \n", time.Now().UTC().Format("2006-01-02")))
	b.WriteString("Audience: Go syntax/grammar-level north-star maintainers  \n")
	b.WriteString("Full machine-readable summary: sibling `.json` with the same basename\n\n")
	b.WriteString("## Claim boundary\n\n")
	b.WriteString(report.ClaimBoundary + "\n\n")
	b.WriteString("## Outcome\n\n")
	b.WriteString("| Gate | Result |\n|------|--------|\n")
	b.WriteString(fmt.Sprintf("| **Full denominator** | **%v** |\n", report.FullDenominator))
	b.WriteString(fmt.Sprintf("| **Render-reparse equal** | **%.2f%%** (%d / %d) |\n",
		report.RenderReparseEqualPercent, report.RenderReparseEqualCount, report.InputDenominator))
	b.WriteString(fmt.Sprintf("| **Accepted (status=passed)** | **%.2f%%** (%d / %d) |\n",
		report.AcceptedPercent, report.SupportedRecordCount, report.InputDenominator))
	b.WriteString(fmt.Sprintf("| **Blocked** | **%d** |\n", report.BlockedRecordCount))
	b.WriteString(fmt.Sprintf("| **Parse-failed excluded** | **%d** |\n", report.ParseFailedCount))
	b.WriteString(fmt.Sprintf("| **Target met (≥95%% full denom)** | **%v** |\n", report.TargetMet))
	b.WriteString(fmt.Sprintf("| **Generator status** | `%s` |\n\n", report.GeneratorStatus))

	b.WriteString("Artifact contract: inventory grammar `")
	b.WriteString(report.GrammarArtifactID)
	b.WriteString("`, stream `")
	b.WriteString(report.StreamContract)
	b.WriteString(fmt.Sprintf("`, layout vocab %d / pad %d.\n\n", report.TokenLayoutVocabSize, report.TokenLayoutPadToken))

	b.WriteString("## Pins\n\n")
	b.WriteString("| Module | Role | Ref | Resolved commit |\n|--------|------|-----|-----------------|\n")
	for _, pin := range report.Pins {
		commit := pin.ResolvedCommit
		if commit == "" {
			commit = "(not recorded)"
		}
		b.WriteString(fmt.Sprintf("| `%s` | %s | `%s` (%s) | `%s` |\n",
			pin.ModulePath, pin.Role, pin.Ref, pin.RefKind, commit))
	}
	b.WriteString("\n")

	b.WriteString("## Per-module coverage\n\n")
	b.WriteString("| Module | Denominator | Accepted | Render-reparse | Blocked |\n|--------|-------------|----------|----------------|---------|\n")
	for _, mod := range report.Modules {
		b.WriteString(fmt.Sprintf("| `%s` | %d | %.2f%% (%d) | %.2f%% (%d) | %d |\n",
			mod.ModuleID, mod.Denominator, mod.AcceptedPercent, mod.SupportedCount,
			mod.RenderReparseEqualPercent, mod.RenderReparseEqualCount, mod.BlockedCount))
	}
	b.WriteString("\n")

	b.WriteString("## Gap buckets\n\n")
	if len(report.GapBuckets) == 0 {
		b.WriteString("No gap buckets (full accepted set).\n\n")
	} else {
		b.WriteString("| Bucket | Count | Sample paths |\n|--------|-------|--------------|\n")
		for _, bucket := range report.GapBuckets {
			sample := strings.Join(bucket.SamplePaths, ", ")
			if sample == "" {
				sample = "—"
			}
			b.WriteString(fmt.Sprintf("| `%s` | %d | %s |\n", bucket.Bucket, bucket.Count, sample))
		}
		b.WriteString("\n")
	}

	if report.FirstBlocker != nil {
		b.WriteString("## First blocker\n\n")
		b.WriteString(fmt.Sprintf("- Path: `%s`\n", report.FirstBlocker.Path))
		b.WriteString(fmt.Sprintf("- Bucket: `%s`\n", report.FirstBlocker.Bucket))
		b.WriteString(fmt.Sprintf("- Reason: %s\n\n", report.FirstBlocker.Reason))
	}

	b.WriteString("## Next ready task\n\n")
	b.WriteString(report.NextReadyTask + "\n\n")

	b.WriteString("## Validation commands\n\n```bash\n")
	for _, cmd := range report.ValidationCommands {
		b.WriteString(cmd + "\n")
	}
	b.WriteString("```\n\n")

	b.WriteString("## Explicit non-claims\n\n")
	for _, claim := range report.NoStableClaims {
		b.WriteString(fmt.Sprintf("- `%s`\n", claim))
	}
	b.WriteString("\n")
	return b.String()
}

// --- self-test ---

func writeTempFile(root, rel, content string) error {
	path := filepath.Join(root, filepath.FromSlash(rel))
	if err := os.MkdirAll(filepath.Dir(path), 0o755); err != nil {
		return err
	}
	return os.WriteFile(path, []byte(content), 0o644)
}

func runSelfTest() error {
	if sanitizeModuleDirName("golang.org/x/tools@v0.38.0") != "golang.org__x__tools__v0.38.0" {
		return fmt.Errorf("unexpected sanitize result: %s", sanitizeModuleDirName("golang.org/x/tools@v0.38.0"))
	}
	if !pathHasPrefix("prometheus/client.go", []string{"prometheus"}) {
		return fmt.Errorf("prefix match failed")
	}
	if pathHasPrefix("api/client.go", []string{"prometheus"}) {
		return fmt.Errorf("prefix exclusion failed")
	}

	tmp, err := os.MkdirTemp("", "tct-go-external-stress-")
	if err != nil {
		return err
	}
	defer os.RemoveAll(tmp)

	modA := filepath.Join(stressModulesDirName, "example.com__mod_a__v1.0.0")
	modB := filepath.Join(stressModulesDirName, "example.com__mod_b__v1.0.0")
	files := map[string]string{
		filepath.Join(modA, "pkg/a.go"): `package pkg

func A() int { return 1 }
`,
		filepath.Join(modA, "pkg/a_test.go"): `package pkg

import "testing"

func TestA(t *testing.T) {
	if A() != 1 {
		t.Fatal("a")
	}
}
`,
		filepath.Join(modA, "pkg/broken.go"): `package pkg

func Broken(
`,
		filepath.Join(modA, "pkg/testdata/skip.go"): "package skip\n",
		filepath.Join(modA, "vendor/v/v.go"):        "package v\n",
		filepath.Join(modA, "pkg/README.md"):        "not go\n",
		filepath.Join(modB, "lib/b.go"): `//go:build linux

package lib

func B() {}
`,
		filepath.Join(modB, "lib/generated.go"): `// Code generated by tct-self-test. DO NOT EDIT.

package lib

func Generated() {}
`,
	}
	for rel, content := range files {
		if err := writeTempFile(tmp, rel, content); err != nil {
			return err
		}
	}

	pins := stressPinsDocument{
		ArtifactKind:      stressPinsArtifactKind,
		Version:           stressVersion,
		Tool:              stressTool,
		ExpectedGoVersion: stressExpectedGo,
		ClaimBoundary:     stressClaimBoundary,
		Modules: []stressPinModule{
			{
				ID:              "example.com/mod_a@v1.0.0",
				ModulePath:      "example.com/mod_a",
				GitURL:          "https://example.com/mod_a.git",
				Ref:             "v1.0.0",
				RefKind:         "tag",
				Role:            "go_tooling_ecosystem",
				CheckoutRelPath: filepath.ToSlash(modA),
				ResolvedCommit:  "aaa",
			},
			{
				ID:              "example.com/mod_b@v1.0.0",
				ModulePath:      "example.com/mod_b",
				GitURL:          "https://example.com/mod_b.git",
				Ref:             "v1.0.0",
				RefKind:         "tag",
				Role:            "bounded_production_tree",
				CheckoutRelPath: filepath.ToSlash(modB),
				ResolvedCommit:  "bbb",
			},
		},
	}
	pinsPath := filepath.Join(tmp, "pins.json")
	if err := writeJSON(pinsPath, pins); err != nil {
		return err
	}
	if err := writeJSON(filepath.Join(tmp, stressResolvedPinsFileName), pins); err != nil {
		return err
	}

	enumPath := filepath.Join(tmp, "enumeration.json")
	if err := runEnumerateMode(pinsPath, tmp, enumPath, ""); err != nil {
		return fmt.Errorf("enumerate: %w", err)
	}
	enumeration, err := loadEnumerationSummary(enumPath)
	if err != nil {
		return err
	}
	// Included: modA a.go, a_test.go, broken.go; modB b.go, generated.go => 5
	if enumeration.FileCount != 5 {
		return fmt.Errorf("file_count = %d, want 5", enumeration.FileCount)
	}
	if enumeration.ParseFailedCount != 1 {
		return fmt.Errorf("parse_failed_count = %d, want 1", enumeration.ParseFailedCount)
	}
	if enumeration.Denominator != 4 {
		return fmt.Errorf("denominator = %d, want 4", enumeration.Denominator)
	}
	if enumeration.ExcludedDirCounts.Testdata != 1 || enumeration.ExcludedDirCounts.Vendor != 1 {
		return fmt.Errorf("excluded dirs = %+v", enumeration.ExcludedDirCounts)
	}
	if enumeration.ClassificationCounts.IsTest != 1 {
		return fmt.Errorf("is_test count = %d, want 1", enumeration.ClassificationCounts.IsTest)
	}
	if enumeration.ClassificationCounts.IsBuildConstrained != 1 {
		return fmt.Errorf("is_build_constrained count = %d, want 1", enumeration.ClassificationCounts.IsBuildConstrained)
	}
	if enumeration.ClassificationCounts.IsGenerated != 1 {
		return fmt.Errorf("is_generated count = %d, want 1", enumeration.ClassificationCounts.IsGenerated)
	}
	if len(enumeration.Modules) != 2 {
		return fmt.Errorf("module summary count = %d, want 2", len(enumeration.Modules))
	}

	// Build a synthetic coverage report over the enumerated records.
	recordsBytes, err := os.ReadFile(enumeration.PerFileRecordsPath)
	if err != nil {
		return err
	}
	var coverageRecords []stressCoverageRecord
	scanner := bufio.NewScanner(strings.NewReader(string(recordsBytes)))
	for scanner.Scan() {
		line := strings.TrimSpace(scanner.Text())
		if line == "" {
			continue
		}
		var fileRec stressFileRecord
		if err := json.Unmarshal([]byte(line), &fileRec); err != nil {
			return err
		}
		rec := stressCoverageRecord{
			Path:        fileRec.Path,
			Bytes:       fileRec.Bytes,
			ParseFailed: fileRec.ParseFailed,
		}
		if fileRec.ParseFailed {
			rec.Status = "parse_failed_excluded"
			rec.Bucket = "parse_failed_excluded_from_denominator"
		} else {
			rec.Attempted = true
			rec.Status = "passed"
			rec.Bucket = "accepted_render_reparse_equal"
			rec.RenderReparseEqual = true
			rec.RawStateMachineComplete = true
			rec.RawTokenCount = 10
			rec.CanonicalTokenCount = 7
			rec.ForcedTokenCount = 3
		}
		coverageRecords = append(coverageRecords, rec)
	}
	if err := scanner.Err(); err != nil {
		return err
	}

	coverage := stressGeneratorCoverageReport{
		ArtifactKind:                  stressCoverageReportKind,
		Status:                        "go_stdlib_syntax_tokenizer_coverage_passed",
		CoverageScope:                 "full_denominator",
		TargetMet:                     true,
		InputFileCount:                5,
		InputDenominator:              4,
		ParseFailedCount:              1,
		AttemptedRecordCount:          4,
		SelectedDenominator:           4,
		SupportedRecordCount:          4,
		BlockedRecordCount:            0,
		RawStateMachineCompleteCount:  4,
		RenderReparseEqualCount:       4,
		AcceptedPercent:               100,
		RenderReparseEqualPercent:     100,
		SelectedAcceptedPercent:       100,
		SelectedRenderReparseEqualPct: 100,
		TotalRawTokenCount:            40,
		TotalCanonicalTokenCount:      28,
		TotalForcedTokenCount:         12,
		TokenLayoutVocabSize:          641,
		TokenLayoutPadToken:           640,
		GrammarArtifactID:             stressGrammarArtifact,
		StreamContract:                stressStreamContract,
		Records:                       coverageRecords,
		FileRecordsSHA256:             "deadbeef",
		GapBuckets:                    []stressCoverageGapBucket{},
	}
	coveragePath := filepath.Join(tmp, "coverage.json")
	if err := writeJSON(coveragePath, coverage); err != nil {
		return err
	}
	outPath := filepath.Join(tmp, "curated.json")
	mdPath := filepath.Join(tmp, "curated.md")
	if err := runSummarizeMode(pinsPath, enumPath, coveragePath, outPath, mdPath); err != nil {
		return fmt.Errorf("summarize: %w", err)
	}
	curatedData, err := os.ReadFile(outPath)
	if err != nil {
		return err
	}
	var curated stressCuratedReport
	if err := json.Unmarshal(curatedData, &curated); err != nil {
		return err
	}
	if curated.ArtifactKind != stressReportArtifactKind {
		return fmt.Errorf("curated artifact_kind = %q", curated.ArtifactKind)
	}
	if !curated.FullDenominator || !curated.TargetMet {
		return fmt.Errorf("expected full denominator target met, got full=%v met=%v", curated.FullDenominator, curated.TargetMet)
	}
	if len(curated.Modules) != 2 {
		return fmt.Errorf("curated modules = %d, want 2", len(curated.Modules))
	}
	md, err := os.ReadFile(mdPath)
	if err != nil {
		return err
	}
	if !strings.Contains(string(md), "Go External Stress Corpus") {
		return fmt.Errorf("markdown missing title")
	}

	// Invalid pins must fail validation.
	badPins := pins
	badPins.Modules = nil
	if err := validatePins(&badPins); err == nil {
		return fmt.Errorf("empty modules should fail validation")
	}
	return nil
}

func main() {
	selfTest := flag.Bool("self-test", false, "run offline self-test")
	fetch := flag.Bool("fetch", false, "shallow-clone pinned modules into --corpus-root")
	enumerate := flag.Bool("enumerate", false, "enumerate fetched modules into coverage-compatible JSONL")
	summarize := flag.Bool("summarize", false, "summarize a generator coverage report into a curated external-stress report")
	pinsPath := flag.String("pins", stressDefaultPinsPath, "path to pinned-module manifest")
	corpusRoot := flag.String("corpus-root", "target/go-external-stress-corpus/v1", "corpus working root under target/")
	output := flag.String("output", "", "JSON output path for enumerate/summarize")
	filesOutput := flag.String("files-output", "", "override per-file JSONL path for enumerate")
	coverageReport := flag.String("coverage-report", "", "generator coverage report JSON for --summarize")
	enumerationPath := flag.String("enumeration", "", "enumeration summary JSON for --summarize")
	markdown := flag.String("markdown", "", "optional markdown path for --summarize")
	flag.Parse()

	modeCount := 0
	for _, enabled := range []bool{*selfTest, *fetch, *enumerate, *summarize} {
		if enabled {
			modeCount++
		}
	}
	if modeCount != 1 {
		exitf("exactly one of --self-test, --fetch, --enumerate, --summarize is required")
	}

	switch {
	case *selfTest:
		if err := runSelfTest(); err != nil {
			exitf("go_external_stress_corpus self-test failed: %v", err)
		}
		fmt.Println("go_external_stress_corpus self-test passed")
	case *fetch:
		if err := runFetchMode(*pinsPath, *corpusRoot); err != nil {
			exitf("go_external_stress_corpus fetch failed: %v", err)
		}
	case *enumerate:
		if err := runEnumerateMode(*pinsPath, *corpusRoot, *output, *filesOutput); err != nil {
			exitf("go_external_stress_corpus enumerate failed: %v", err)
		}
	case *summarize:
		if *coverageReport == "" || *enumerationPath == "" {
			exitf("--summarize requires --coverage-report and --enumeration")
		}
		if err := runSummarizeMode(*pinsPath, *enumerationPath, *coverageReport, *output, *markdown); err != nil {
			exitf("go_external_stress_corpus summarize failed: %v", err)
		}
	}
}
