// go_syntax_corpus_select ranks parser-backed Go source candidates for the
// next syntax-token training corpus tranche.
package main

import (
	"crypto/sha256"
	"encoding/hex"
	"encoding/json"
	"errors"
	"flag"
	"fmt"
	"go/ast"
	"go/parser"
	"go/token"
	"io"
	"os"
	"path/filepath"
	"runtime"
	"sort"
	"strings"
)

const (
	goSyntaxSelectionArtifactKind = "tct_go_syntax_corpus_selection_v1"
	goSyntaxSelectionVersion      = 1
	goSyntaxSelectionTool         = "go_syntax_corpus_select"
	goSyntaxSelectionExpectedGo   = "go1.24.3"
	goSyntaxVersionCheckMatched   = "matched"
	goSyntaxVersionCheckOverride  = "mismatch_overridden"
	goSyntaxVersionCheckSkipped   = "not_go_toolchain_revision"
	goSyntaxNoStableClaim         = "parser-backed source-selection evidence only; no generated-codec route, source-validity, arena IR, or stable backend promotion"
)

var goSyntaxSelectionBlockedClaims = []string{
	"no_broad_corpus_claim",
	"no_source_validity_by_construction_claim",
	"no_type_correctness_claim",
	"no_model_quality_claim",
	"no_paper_facing_claim",
	"no_generated_hot_path_claim",
	"no_stable_backend_claim",
	"no_arena_ir_promotion",
}

type goSyntaxSelectionConfig struct {
	SourceRoot                     string
	Output                         string
	CorpusID                       string
	MaxFiles                       int
	MaxFileBytes                   int64
	MaxTotalBytes                  int64
	License                        string
	LicenseTextPath                string
	SourceRevision                 string
	SourceURIPrefix                string
	AllowGoVersionMismatch         bool
	ExistingManifest               string
	ExcludeReportIngestionManifest string
	ExcludeExisting                bool
	ExcludeGenerated               bool
	ExcludeCgo                     bool
	DedupeShapes                   bool
	IncludeTestFiles               bool
	IncludeTestdata                bool
	ExcludePrefixes                []string
	SelectionNote                  string
}

type goSyntaxSelectionReport struct {
	ArtifactKind             string                                     `json:"artifact_kind"`
	Version                  int                                        `json:"version"`
	Tool                     string                                     `json:"tool"`
	GoVersion                string                                     `json:"go_version"`
	GoRoot                   string                                     `json:"go_root"`
	GoVersionCheck           string                                     `json:"go_version_check"`
	Status                   string                                     `json:"status"`
	CorpusID                 string                                     `json:"corpus_id"`
	SourceRoot               string                                     `json:"source_root"`
	SourceRevision           string                                     `json:"source_revision"`
	SourceURIPrefix          string                                     `json:"source_uri_prefix,omitempty"`
	License                  string                                     `json:"license"`
	LicenseTextPath          string                                     `json:"license_text_path,omitempty"`
	SelectionNote            string                                     `json:"selection_note"`
	SelectionPolicy          goSyntaxSelectionPolicy                    `json:"selection_policy"`
	SourceFileCount          int                                        `json:"source_file_count"`
	CandidateCount           int                                        `json:"candidate_count"`
	SelectedCount            int                                        `json:"selected_count"`
	ExcludedCount            int                                        `json:"excluded_count"`
	CandidateBytes           int64                                      `json:"candidate_bytes"`
	SelectedBytes            int64                                      `json:"selected_bytes"`
	ExistingCandidates       int                                        `json:"existing_candidates"`
	NewCandidates            int                                        `json:"new_candidates"`
	SourceListSHA256         string                                     `json:"source_list_sha256"`
	SelectedListSHA256       string                                     `json:"selected_list_sha256"`
	ExistingManifest         *goSyntaxSelectionExistingManifest         `json:"existing_manifest,omitempty"`
	ReportIngestionExclusion *goSyntaxSelectionReportIngestionExclusion `json:"report_ingestion_exclusion,omitempty"`
	PackageCoverage          []goSyntaxSelectionPackageSummary          `json:"package_coverage"`
	ConstructCoverage        []goSyntaxSelectionConstruct               `json:"construct_coverage"`
	SelectedCandidates       []goSyntaxSelectionCandidate               `json:"selected_candidates"`
	Candidates               []goSyntaxSelectionCandidate               `json:"candidates"`
	Exclusions               []goSyntaxSelectionExclusion               `json:"exclusions"`
	ClaimBoundaries          goSyntaxSelectionClaimBoundaries           `json:"claim_boundaries"`
}

type goSyntaxSelectionPolicy struct {
	MaxFiles                       int      `json:"max_files"`
	MaxFileBytes                   int64    `json:"max_file_bytes"`
	MaxTotalBytes                  int64    `json:"max_total_bytes"`
	ExistingManifest               string   `json:"existing_manifest,omitempty"`
	ExcludeReportIngestionManifest string   `json:"exclude_report_ingestion_manifest,omitempty"`
	ExcludeExisting                bool     `json:"exclude_existing"`
	ExcludeGenerated               bool     `json:"exclude_generated"`
	ExcludeCgo                     bool     `json:"exclude_cgo"`
	DedupeShapes                   bool     `json:"dedupe_shapes"`
	IncludeTestFiles               bool     `json:"include_test_files"`
	IncludeTestdata                bool     `json:"include_testdata"`
	AllowGoVersionMismatch         bool     `json:"allow_go_version_mismatch"`
	ExcludePrefixes                []string `json:"exclude_prefixes"`
	ScorePolicy                    string   `json:"score_policy"`
}

type goSyntaxSelectionClaimBoundaries struct {
	Boundary              string   `json:"boundary"`
	NoStableClaim         string   `json:"no_stable_claim"`
	BlockedClaims         []string `json:"blocked_claims"`
	ArenaIRPromotion      bool     `json:"arena_ir_promotion_allowed"`
	GeneratedCodecChanged bool     `json:"generated_codec_changed"`
}

type goSyntaxSelectionExistingManifest struct {
	Path                string `json:"path"`
	SHA256              string `json:"sha256"`
	EntryCount          int    `json:"entry_count"`
	ManifestPathsSHA256 string `json:"manifest_paths_sha256"`
}

type goSyntaxSelectionReportIngestionExclusion struct {
	Path                string `json:"path"`
	SHA256              string `json:"sha256"`
	RecordCount         int    `json:"record_count"`
	ManifestPathsSHA256 string `json:"manifest_paths_sha256"`
	SourceSHA256sSHA256 string `json:"source_sha256s_sha256"`
}

type goSyntaxSelectionCandidate struct {
	ManifestPath          string                       `json:"manifest_path"`
	SourcePath            string                       `json:"source_path"`
	SourceURI             string                       `json:"source_uri,omitempty"`
	File                  goSyntaxSelectionFile        `json:"file"`
	Package               string                       `json:"package"`
	Imports               []string                     `json:"imports"`
	TopLevelDecls         int                          `json:"top_level_decls"`
	FunctionDecls         int                          `json:"function_decls"`
	MethodDecls           int                          `json:"method_decls"`
	CommentGroups         int                          `json:"comment_groups"`
	UniqueConstructs      []string                     `json:"unique_constructs"`
	Constructs            []goSyntaxSelectionConstruct `json:"constructs"`
	ShapeSHA256           string                       `json:"shape_sha256"`
	ExistingManifestEntry bool                         `json:"existing_manifest_entry"`
	SelectionScore        int                          `json:"selection_score"`
	SelectionOrder        int                          `json:"selection_order,omitempty"`
	Selected              bool                         `json:"selected"`
	SelectionReason       string                       `json:"selection_reason"`
}

type goSyntaxSelectionFile struct {
	Path   string `json:"path"`
	SHA256 string `json:"sha256"`
	Bytes  int64  `json:"bytes"`
}

type goSyntaxSelectionConstruct struct {
	Construct string   `json:"construct"`
	Count     int      `json:"count"`
	FileCount int      `json:"file_count,omitempty"`
	Files     []string `json:"files,omitempty"`
}

type goSyntaxSelectionPackageSummary struct {
	Package   string   `json:"package"`
	FileCount int      `json:"file_count"`
	Bytes     int64    `json:"bytes"`
	Files     []string `json:"files"`
}

type goSyntaxSelectionExclusion struct {
	ManifestPath string                 `json:"manifest_path"`
	SourcePath   string                 `json:"source_path"`
	Reason       string                 `json:"reason"`
	Detail       string                 `json:"detail,omitempty"`
	File         *goSyntaxSelectionFile `json:"file,omitempty"`
}

func main() {
	if err := runGoSyntaxSelection(os.Args[1:], os.Stdout); err != nil {
		fmt.Fprintf(os.Stderr, "%v\n", err)
		os.Exit(1)
	}
}

func runGoSyntaxSelection(args []string, stdout io.Writer) error {
	config := defaultGoSyntaxSelectionConfig()
	excludePrefixes := strings.Join(config.ExcludePrefixes, ",")
	flags := flag.NewFlagSet(goSyntaxSelectionTool, flag.ContinueOnError)
	flags.SetOutput(stdout)
	flags.StringVar(&config.SourceRoot, "source-root", config.SourceRoot, "Go source root or fixture root to scan")
	flags.StringVar(&config.Output, "output", "", "write JSON selection report to this path instead of stdout")
	flags.StringVar(&config.CorpusID, "corpus-id", config.CorpusID, "corpus identifier recorded in the report")
	flags.IntVar(&config.MaxFiles, "max-files", config.MaxFiles, "maximum selected candidate files")
	flags.Int64Var(&config.MaxFileBytes, "max-file-bytes", config.MaxFileBytes, "maximum bytes per candidate file")
	flags.Int64Var(&config.MaxTotalBytes, "max-total-bytes", config.MaxTotalBytes, "maximum selected candidate bytes")
	flags.StringVar(&config.License, "license", config.License, "source license label recorded for candidates")
	flags.StringVar(&config.LicenseTextPath, "license-text", config.LicenseTextPath, "path to source license text")
	flags.StringVar(&config.SourceRevision, "source-revision", config.SourceRevision, "source revision or tag recorded for provenance")
	flags.StringVar(&config.SourceURIPrefix, "source-uri-prefix", config.SourceURIPrefix, "URI prefix used to form candidate source_uri values")
	flags.BoolVar(&config.AllowGoVersionMismatch, "allow-go-version-mismatch", config.AllowGoVersionMismatch, "allow runtime Go version to differ from --source-revision when scanning provisional evidence")
	flags.StringVar(&config.ExistingManifest, "existing-manifest", "", "checked Go syntax corpus manifest whose entries should be marked as existing")
	flags.StringVar(&config.ExcludeReportIngestionManifest, "exclude-report-ingestion-manifest", "", "checked report-ingestion manifest whose source records should be excluded from selection")
	flags.BoolVar(&config.ExcludeExisting, "exclude-existing", config.ExcludeExisting, "exclude files already present in --existing-manifest")
	flags.BoolVar(&config.ExcludeGenerated, "exclude-generated", config.ExcludeGenerated, "exclude Go files carrying the standard generated-file marker")
	flags.BoolVar(&config.ExcludeCgo, "exclude-cgo", config.ExcludeCgo, "exclude files importing C")
	flags.BoolVar(&config.DedupeShapes, "dedupe-shapes", config.DedupeShapes, "exclude later files with identical package and construct shape")
	flags.BoolVar(&config.IncludeTestFiles, "include-test-files", config.IncludeTestFiles, "include *_test.go files as candidates")
	flags.BoolVar(&config.IncludeTestdata, "include-testdata", config.IncludeTestdata, "include directories named testdata")
	flags.StringVar(&excludePrefixes, "exclude-prefixes", excludePrefixes, "comma-separated manifest-path prefixes excluded from selection")
	flags.StringVar(&config.SelectionNote, "selection-note", config.SelectionNote, "selection note recorded in the report")

	if err := flags.Parse(args); err != nil {
		if errors.Is(err, flag.ErrHelp) {
			return nil
		}
		return err
	}
	config.ExcludePrefixes = splitGoSyntaxSelectionCSV(excludePrefixes)

	report, err := buildGoSyntaxSelectionReport(config)
	if err != nil {
		return err
	}
	return writeGoSyntaxSelectionJSON(config.Output, stdout, report)
}

func defaultGoSyntaxSelectionConfig() goSyntaxSelectionConfig {
	return goSyntaxSelectionConfig{
		SourceRoot:       "tests/fixtures/go_syntax_corpus/src",
		CorpusID:         "go-syntax-corpus-selection-v1",
		MaxFiles:         32,
		MaxFileBytes:     16 * 1024,
		MaxTotalBytes:    128 * 1024,
		License:          "BSD-3-Clause",
		LicenseTextPath:  "tests/fixtures/go_syntax_corpus/LICENSE.go-BSD-3-Clause.txt",
		SourceRevision:   goSyntaxSelectionExpectedGo,
		SourceURIPrefix:  "https://cs.opensource.google/go/go/+/refs/tags/go1.24.3:src",
		ExcludeGenerated: true,
		ExcludeCgo:       true,
		DedupeShapes:     false,
		IncludeTestFiles: true,
		IncludeTestdata:  false,
		ExcludePrefixes:  []string{"cmd/"},
		SelectionNote: "Parser-backed Go source-selection evidence for a bounded " +
			"syntax-token training corpus tranche; generated-codec ingestion remains " +
			"the next step.",
	}
}

func buildGoSyntaxSelectionReport(config goSyntaxSelectionConfig) (*goSyntaxSelectionReport, error) {
	if config.SourceRoot == "" {
		return nil, errors.New("--source-root is required")
	}
	if config.MaxFiles <= 0 {
		return nil, errors.New("--max-files must be positive")
	}
	if config.MaxFileBytes <= 0 {
		return nil, errors.New("--max-file-bytes must be positive")
	}
	if config.MaxTotalBytes <= 0 {
		return nil, errors.New("--max-total-bytes must be positive")
	}
	if config.ExcludeExisting && config.ExistingManifest == "" {
		return nil, errors.New("--exclude-existing requires --existing-manifest")
	}
	goVersionCheck, err := validateGoSyntaxSelectionVersion(config.SourceRevision, config.AllowGoVersionMismatch)
	if err != nil {
		return nil, err
	}

	info, err := os.Stat(config.SourceRoot)
	if err != nil {
		return nil, fmt.Errorf("stat source root %s: %w", config.SourceRoot, err)
	}
	if !info.IsDir() {
		return nil, fmt.Errorf("source root %s is not a directory", config.SourceRoot)
	}
	existingPaths, existingManifest, err := loadGoSyntaxExistingManifest(config.ExistingManifest)
	if err != nil {
		return nil, err
	}
	reportIngestionPaths, reportIngestionSHAs, reportIngestionExclusion, err := loadGoSyntaxReportIngestionExclusion(config.ExcludeReportIngestionManifest)
	if err != nil {
		return nil, err
	}

	var candidates []goSyntaxSelectionCandidate
	var exclusions []goSyntaxSelectionExclusion
	sourceFiles := 0
	shapeOwners := map[string]string{}

	err = filepath.WalkDir(config.SourceRoot, func(path string, entry os.DirEntry, walkErr error) error {
		if walkErr != nil {
			return walkErr
		}
		if entry.IsDir() {
			if path != config.SourceRoot && shouldSkipGoSyntaxSelectionDir(entry.Name(), config.IncludeTestdata) {
				return filepath.SkipDir
			}
			return nil
		}
		if !strings.HasSuffix(entry.Name(), ".go") {
			return nil
		}
		sourceFiles++
		manifestPath, err := filepath.Rel(config.SourceRoot, path)
		if err != nil {
			return fmt.Errorf("relativize %s: %w", path, err)
		}
		manifestPath = filepath.ToSlash(manifestPath)
		sourcePath := filepath.ToSlash(path)
		if !config.IncludeTestFiles && strings.HasSuffix(entry.Name(), "_test.go") {
			exclusions = append(exclusions, goSyntaxSelectionExclusion{
				ManifestPath: manifestPath,
				SourcePath:   sourcePath,
				Reason:       "test_file",
				Detail:       "test files disabled by selection policy",
			})
			return nil
		}
		if prefix, excluded := goSyntaxSelectionExcludedPrefix(manifestPath, config.ExcludePrefixes); excluded {
			exclusions = append(exclusions, goSyntaxSelectionExclusion{
				ManifestPath: manifestPath,
				SourcePath:   sourcePath,
				Reason:       "excluded_prefix",
				Detail:       "manifest path matches excluded prefix " + prefix,
			})
			return nil
		}
		if config.ExcludeExisting && existingPaths[manifestPath] {
			exclusions = append(exclusions, goSyntaxSelectionExclusion{
				ManifestPath: manifestPath,
				SourcePath:   sourcePath,
				Reason:       "existing_manifest_entry",
				Detail:       "manifest path already exists in " + filepath.ToSlash(config.ExistingManifest),
			})
			return nil
		}
		if reportIngestionPaths[manifestPath] {
			exclusions = append(exclusions, goSyntaxSelectionExclusion{
				ManifestPath: manifestPath,
				SourcePath:   sourcePath,
				Reason:       "report_ingestion_manifest_record",
				Detail:       "manifest path already exists in " + filepath.ToSlash(config.ExcludeReportIngestionManifest),
			})
			return nil
		}
		candidate, exclusion, err := inspectGoSyntaxSelectionFile(config, path)
		if err != nil {
			return err
		}
		if exclusion != nil {
			exclusions = append(exclusions, *exclusion)
			return nil
		}
		if config.DedupeShapes {
			if owner, exists := shapeOwners[candidate.ShapeSHA256]; exists {
				exclusions = append(exclusions, goSyntaxSelectionExclusion{
					ManifestPath: candidate.ManifestPath,
					SourcePath:   candidate.SourcePath,
					Reason:       "duplicate_shape",
					Detail:       "same package and construct shape as " + owner,
					File:         &candidate.File,
				})
				return nil
			}
			shapeOwners[candidate.ShapeSHA256] = candidate.ManifestPath
		}
		if reportIngestionSHAs[candidate.File.SHA256] {
			exclusions = append(exclusions, goSyntaxSelectionExclusion{
				ManifestPath: candidate.ManifestPath,
				SourcePath:   candidate.SourcePath,
				Reason:       "report_ingestion_manifest_record",
				Detail:       "source sha256 already exists in " + filepath.ToSlash(config.ExcludeReportIngestionManifest),
				File:         &candidate.File,
			})
			return nil
		}
		candidate.ExistingManifestEntry = existingPaths[candidate.ManifestPath]
		candidates = append(candidates, candidate)
		return nil
	})
	if err != nil {
		return nil, fmt.Errorf("walk source root %s: %w", config.SourceRoot, err)
	}

	sort.Slice(candidates, func(i, j int) bool {
		return candidates[i].ManifestPath < candidates[j].ManifestPath
	})
	sort.Slice(exclusions, func(i, j int) bool {
		if exclusions[i].Reason != exclusions[j].Reason {
			return exclusions[i].Reason < exclusions[j].Reason
		}
		return exclusions[i].ManifestPath < exclusions[j].ManifestPath
	})

	selected, selectedBytes := selectGoSyntaxCandidates(candidates, config)
	selectedByPath := map[string]goSyntaxSelectionCandidate{}
	for _, candidate := range selected {
		selectedByPath[candidate.ManifestPath] = candidate
	}

	candidateBytes := int64(0)
	existingCandidates := 0
	for i := range candidates {
		candidateBytes += candidates[i].File.Bytes
		if candidates[i].ExistingManifestEntry {
			existingCandidates++
		}
		if selectedCandidate, ok := selectedByPath[candidates[i].ManifestPath]; ok {
			candidates[i] = selectedCandidate
		} else {
			candidates[i].SelectionScore = scoreGoSyntaxCandidate(candidates[i], nil, nil)
			candidates[i].SelectionReason = "candidate not selected within bounded coverage budget"
		}
	}

	status := "ready"
	if len(candidates) == 0 {
		status = "blocked_no_candidates"
	} else if len(selected) == 0 {
		status = "blocked_no_selected_candidates"
	}

	return &goSyntaxSelectionReport{
		ArtifactKind:    goSyntaxSelectionArtifactKind,
		Version:         goSyntaxSelectionVersion,
		Tool:            goSyntaxSelectionTool,
		GoVersion:       runtime.Version(),
		GoRoot:          filepath.ToSlash(runtime.GOROOT()),
		GoVersionCheck:  goVersionCheck,
		Status:          status,
		CorpusID:        config.CorpusID,
		SourceRoot:      filepath.ToSlash(config.SourceRoot),
		SourceRevision:  config.SourceRevision,
		SourceURIPrefix: strings.TrimRight(config.SourceURIPrefix, "/"),
		License:         config.License,
		LicenseTextPath: filepath.ToSlash(config.LicenseTextPath),
		SelectionNote:   config.SelectionNote,
		SelectionPolicy: goSyntaxSelectionPolicy{
			MaxFiles:                       config.MaxFiles,
			MaxFileBytes:                   config.MaxFileBytes,
			MaxTotalBytes:                  config.MaxTotalBytes,
			ExistingManifest:               filepath.ToSlash(config.ExistingManifest),
			ExcludeReportIngestionManifest: filepath.ToSlash(config.ExcludeReportIngestionManifest),
			ExcludeExisting:                config.ExcludeExisting,
			ExcludeGenerated:               config.ExcludeGenerated,
			ExcludeCgo:                     config.ExcludeCgo,
			DedupeShapes:                   config.DedupeShapes,
			IncludeTestFiles:               config.IncludeTestFiles,
			IncludeTestdata:                config.IncludeTestdata,
			AllowGoVersionMismatch:         config.AllowGoVersionMismatch,
			ExcludePrefixes:                append([]string(nil), config.ExcludePrefixes...),
			ScorePolicy: "greedy coverage gain by previously unseen syntax " +
				"constructs, package diversity, unique-construct count, and byte cost",
		},
		SourceFileCount:          sourceFiles,
		CandidateCount:           len(candidates),
		SelectedCount:            len(selected),
		ExcludedCount:            len(exclusions),
		CandidateBytes:           candidateBytes,
		SelectedBytes:            selectedBytes,
		ExistingCandidates:       existingCandidates,
		NewCandidates:            len(candidates) - existingCandidates,
		SourceListSHA256:         hashGoSyntaxSelectionList(candidatePaths(candidates)),
		SelectedListSHA256:       hashGoSyntaxSelectionList(candidatePaths(selected)),
		ExistingManifest:         existingManifest,
		ReportIngestionExclusion: reportIngestionExclusion,
		PackageCoverage:          summarizeGoSyntaxPackages(candidates),
		ConstructCoverage:        summarizeGoSyntaxConstructs(candidates),
		SelectedCandidates:       selected,
		Candidates:               candidates,
		Exclusions:               exclusions,
		ClaimBoundaries: goSyntaxSelectionClaimBoundaries{
			Boundary: "parser-backed metadata -> GrammarIR -> StructuralModel -> " +
				"token layout -> generated codec remains the syntax-level path; this " +
				"report only selects source candidates before generated-codec ingestion",
			NoStableClaim:         goSyntaxNoStableClaim,
			BlockedClaims:         append([]string(nil), goSyntaxSelectionBlockedClaims...),
			ArenaIRPromotion:      false,
			GeneratedCodecChanged: false,
		},
	}, nil
}

func validateGoSyntaxSelectionVersion(sourceRevision string, allowMismatch bool) (string, error) {
	if !strings.HasPrefix(sourceRevision, "go") {
		return goSyntaxVersionCheckSkipped, nil
	}
	if runtime.Version() == sourceRevision {
		return goSyntaxVersionCheckMatched, nil
	}
	if allowMismatch {
		return goSyntaxVersionCheckOverride, nil
	}
	return "", fmt.Errorf(
		"GO VERSION MISMATCH: source revision %s but selection helper is running under %s; "+
			"the selection report is only authoritative against the pinned %s toolchain; "+
			"pass --allow-go-version-mismatch only for provisional evidence",
		sourceRevision, runtime.Version(), goSyntaxSelectionExpectedGo)
}

func inspectGoSyntaxSelectionFile(config goSyntaxSelectionConfig, path string) (goSyntaxSelectionCandidate, *goSyntaxSelectionExclusion, error) {
	manifestPath, err := filepath.Rel(config.SourceRoot, path)
	if err != nil {
		return goSyntaxSelectionCandidate{}, nil, fmt.Errorf("relativize %s: %w", path, err)
	}
	manifestPath = filepath.ToSlash(manifestPath)
	sourcePath := filepath.ToSlash(path)

	source, err := os.ReadFile(path)
	if err != nil {
		return goSyntaxSelectionCandidate{}, nil, fmt.Errorf("read %s: %w", path, err)
	}
	file := goSyntaxSelectionFile{
		Path:   sourcePath,
		SHA256: hashGoSyntaxSelectionBytes(source),
		Bytes:  int64(len(source)),
	}
	if int64(len(source)) > config.MaxFileBytes {
		return goSyntaxSelectionCandidate{}, &goSyntaxSelectionExclusion{
			ManifestPath: manifestPath,
			SourcePath:   sourcePath,
			Reason:       "too_large",
			Detail:       fmt.Sprintf("%d bytes exceeds max-file-bytes %d", len(source), config.MaxFileBytes),
			File:         &file,
		}, nil
	}
	if config.ExcludeGenerated && hasGoSyntaxGeneratedMarker(source) {
		return goSyntaxSelectionCandidate{}, &goSyntaxSelectionExclusion{
			ManifestPath: manifestPath,
			SourcePath:   sourcePath,
			Reason:       "generated_file",
			Detail:       "standard generated-file marker found before package clause",
			File:         &file,
		}, nil
	}

	fset := token.NewFileSet()
	parsed, err := parser.ParseFile(fset, path, source, parser.ParseComments)
	if err != nil {
		return goSyntaxSelectionCandidate{}, &goSyntaxSelectionExclusion{
			ManifestPath: manifestPath,
			SourcePath:   sourcePath,
			Reason:       "parser_error",
			Detail:       err.Error(),
			File:         &file,
		}, nil
	}
	if config.ExcludeCgo && hasGoSyntaxCgoImport(parsed) {
		return goSyntaxSelectionCandidate{}, &goSyntaxSelectionExclusion{
			ManifestPath: manifestPath,
			SourcePath:   sourcePath,
			Reason:       "cgo_import",
			Detail:       "file imports C",
			File:         &file,
		}, nil
	}

	counts := collectGoSyntaxConstructs(parsed)
	constructs := goSyntaxConstructCounts(counts, true)
	uniqueConstructs := make([]string, 0, len(constructs))
	for _, construct := range constructs {
		uniqueConstructs = append(uniqueConstructs, construct.Construct)
	}
	functionDecls, methodDecls := countGoSyntaxFunctions(parsed)
	imports := goSyntaxImportPaths(parsed)

	return goSyntaxSelectionCandidate{
		ManifestPath:     manifestPath,
		SourcePath:       sourcePath,
		SourceURI:        goSyntaxSourceURI(config.SourceURIPrefix, manifestPath),
		File:             file,
		Package:          parsed.Name.Name,
		Imports:          imports,
		TopLevelDecls:    len(parsed.Decls),
		FunctionDecls:    functionDecls,
		MethodDecls:      methodDecls,
		CommentGroups:    len(parsed.Comments),
		UniqueConstructs: uniqueConstructs,
		Constructs:       constructs,
		ShapeSHA256:      goSyntaxShapeHash(parsed.Name.Name, uniqueConstructs),
	}, nil, nil
}

func shouldSkipGoSyntaxSelectionDir(name string, includeTestdata bool) bool {
	if strings.HasPrefix(name, ".") || name == "vendor" {
		return true
	}
	return !includeTestdata && name == "testdata"
}

func splitGoSyntaxSelectionCSV(value string) []string {
	parts := strings.Split(value, ",")
	result := make([]string, 0, len(parts))
	for _, part := range parts {
		normalized := strings.TrimSpace(filepath.ToSlash(part))
		if normalized == "" {
			continue
		}
		result = append(result, normalized)
	}
	sort.Strings(result)
	return result
}

func goSyntaxSelectionExcludedPrefix(manifestPath string, prefixes []string) (string, bool) {
	for _, prefix := range prefixes {
		if strings.HasPrefix(manifestPath, prefix) {
			return prefix, true
		}
	}
	return "", false
}

type goSyntaxExistingManifestJSON struct {
	Entries []struct {
		ManifestPath string `json:"manifest_path"`
	} `json:"entries"`
}

type goSyntaxReportIngestionManifestJSON struct {
	Records []struct {
		ManifestPath string `json:"manifest_path"`
		SourceSHA256 string `json:"source_sha256"`
	} `json:"records"`
}

func loadGoSyntaxExistingManifest(path string) (map[string]bool, *goSyntaxSelectionExistingManifest, error) {
	if path == "" {
		return map[string]bool{}, nil, nil
	}
	data, err := os.ReadFile(path)
	if err != nil {
		return nil, nil, fmt.Errorf("read existing manifest %s: %w", path, err)
	}
	var manifest goSyntaxExistingManifestJSON
	if err := json.Unmarshal(data, &manifest); err != nil {
		return nil, nil, fmt.Errorf("parse existing manifest %s: %w", path, err)
	}
	paths := make([]string, 0, len(manifest.Entries))
	seen := map[string]bool{}
	for _, entry := range manifest.Entries {
		manifestPath := strings.TrimSpace(filepath.ToSlash(entry.ManifestPath))
		if manifestPath == "" {
			return nil, nil, fmt.Errorf("existing manifest %s contains empty manifest_path", path)
		}
		seen[manifestPath] = true
		paths = append(paths, manifestPath)
	}
	sort.Strings(paths)
	return seen, &goSyntaxSelectionExistingManifest{
		Path:                filepath.ToSlash(path),
		SHA256:              hashGoSyntaxSelectionBytes(data),
		EntryCount:          len(paths),
		ManifestPathsSHA256: hashGoSyntaxSelectionList(paths),
	}, nil
}

func loadGoSyntaxReportIngestionExclusion(path string) (map[string]bool, map[string]bool, *goSyntaxSelectionReportIngestionExclusion, error) {
	if path == "" {
		return map[string]bool{}, map[string]bool{}, nil, nil
	}
	data, err := os.ReadFile(path)
	if err != nil {
		return nil, nil, nil, fmt.Errorf("read report-ingestion exclusion manifest %s: %w", path, err)
	}
	var manifest goSyntaxReportIngestionManifestJSON
	if err := json.Unmarshal(data, &manifest); err != nil {
		return nil, nil, nil, fmt.Errorf("parse report-ingestion exclusion manifest %s: %w", path, err)
	}
	paths := make([]string, 0, len(manifest.Records))
	shas := make([]string, 0, len(manifest.Records))
	pathSet := map[string]bool{}
	shaSet := map[string]bool{}
	for _, record := range manifest.Records {
		manifestPath := strings.TrimSpace(filepath.ToSlash(record.ManifestPath))
		if manifestPath == "" {
			return nil, nil, nil, fmt.Errorf("report-ingestion exclusion manifest %s contains empty manifest_path", path)
		}
		pathSet[manifestPath] = true
		paths = append(paths, manifestPath)
		sourceSHA := strings.TrimSpace(record.SourceSHA256)
		if sourceSHA != "" {
			shaSet[sourceSHA] = true
			shas = append(shas, sourceSHA)
		}
	}
	sort.Strings(paths)
	sort.Strings(shas)
	return pathSet, shaSet, &goSyntaxSelectionReportIngestionExclusion{
		Path:                filepath.ToSlash(path),
		SHA256:              hashGoSyntaxSelectionBytes(data),
		RecordCount:         len(paths),
		ManifestPathsSHA256: hashGoSyntaxSelectionList(paths),
		SourceSHA256sSHA256: hashGoSyntaxSelectionList(shas),
	}, nil
}

func hasGoSyntaxGeneratedMarker(source []byte) bool {
	limit := len(source)
	if limit > 4096 {
		limit = 4096
	}
	prefix := string(source[:limit])
	packageIndex := strings.Index(prefix, "\npackage ")
	if packageIndex >= 0 {
		prefix = prefix[:packageIndex]
	}
	return strings.Contains(prefix, "Code generated") && strings.Contains(prefix, "DO NOT EDIT")
}

func hasGoSyntaxCgoImport(file *ast.File) bool {
	for _, importSpec := range file.Imports {
		if importSpec.Path != nil && importSpec.Path.Value == `"C"` {
			return true
		}
	}
	return false
}

func collectGoSyntaxConstructs(file *ast.File) map[string]int {
	counts := map[string]int{
		"file":           1,
		"package_clause": 1,
	}
	topLevelDecls := map[*ast.GenDecl]bool{}
	for _, decl := range file.Decls {
		if genDecl, ok := decl.(*ast.GenDecl); ok {
			topLevelDecls[genDecl] = true
		}
	}
	ast.Inspect(file, func(node ast.Node) bool {
		switch n := node.(type) {
		case *ast.GenDecl:
			switch n.Tok {
			case token.IMPORT:
				counts["import_declaration"]++
			case token.CONST:
				counts["const_declaration"]++
			case token.TYPE:
				counts["type_declaration"]++
			case token.VAR:
				if topLevelDecls[n] || n.Lparen.IsValid() {
					counts["var_declaration"]++
				} else {
					counts["local_var_declaration"]++
				}
			}
		case *ast.FuncDecl:
			if n.Recv != nil {
				counts["method_declaration"]++
			} else {
				counts["function_declaration"]++
			}
		case *ast.ImportSpec:
			counts["import_spec"]++
		case *ast.ValueSpec:
			counts["value_spec"]++
		case *ast.TypeSpec:
			counts["type_spec"]++
		case *ast.Field:
			counts["field"]++
		case *ast.FieldList:
			counts["field_list"]++
		case *ast.BlockStmt:
			counts["block_statement"]++
		case *ast.ReturnStmt:
			counts["return_statement"]++
		case *ast.AssignStmt:
			switch n.Tok {
			case token.DEFINE:
				counts["short_var_declaration"]++
			case token.ASSIGN:
				counts["assignment_statement"]++
			default:
				counts["compound_assignment_statement"]++
			}
		case *ast.IncDecStmt:
			counts["increment_decrement_statement"]++
		case *ast.BranchStmt:
			counts["branch_statement"]++
		case *ast.IfStmt:
			counts["if_statement"]++
		case *ast.ForStmt:
			counts["for_statement"]++
		case *ast.RangeStmt:
			counts["range_statement"]++
		case *ast.SwitchStmt, *ast.TypeSwitchStmt:
			counts["switch_statement"]++
		case *ast.CaseClause:
			counts["case_clause"]++
		case *ast.DeclStmt:
			counts["declaration_statement"]++
		case *ast.GoStmt:
			counts["go_statement"]++
		case *ast.DeferStmt:
			counts["defer_statement"]++
		case *ast.SendStmt:
			counts["send_statement"]++
		case *ast.SelectStmt:
			counts["select_statement"]++
		case *ast.CallExpr:
			counts["call_expression"]++
		case *ast.SelectorExpr:
			counts["selector_expression"]++
		case *ast.Ident:
			counts["identifier_expression"]++
		case *ast.BasicLit:
			switch n.Kind {
			case token.INT:
				counts["basic_lit_integer"]++
			case token.STRING, token.CHAR:
				counts["basic_lit_string"]++
			default:
				counts["basic_lit_other"]++
			}
		case *ast.BinaryExpr:
			counts["binary_expression"]++
		case *ast.UnaryExpr:
			counts["unary_expression"]++
		case *ast.ParenExpr:
			counts["parenthesized_expression"]++
		case *ast.CompositeLit:
			counts["composite_literal"]++
		case *ast.KeyValueExpr:
			counts["key_value_expression"]++
		case *ast.ArrayType:
			counts["array_type"]++
		case *ast.IndexExpr, *ast.IndexListExpr:
			counts["index_expression"]++
		case *ast.SliceExpr:
			counts["slice_expression"]++
		case *ast.StarExpr:
			counts["star_expression"]++
		case *ast.StructType:
			counts["struct_type"]++
		case *ast.InterfaceType:
			counts["interface_type"]++
		case *ast.FuncLit:
			counts["function_literal"]++
		case *ast.FuncType:
			counts["function_type"]++
		case *ast.MapType:
			counts["map_type"]++
		case *ast.ChanType:
			counts["channel_type"]++
		case *ast.TypeAssertExpr:
			counts["type_assertion_expression"]++
		}
		return true
	})
	return counts
}

func countGoSyntaxFunctions(file *ast.File) (int, int) {
	functions := 0
	methods := 0
	for _, decl := range file.Decls {
		if fn, ok := decl.(*ast.FuncDecl); ok {
			if fn.Recv == nil {
				functions++
			} else {
				methods++
			}
		}
	}
	return functions, methods
}

func goSyntaxImportPaths(file *ast.File) []string {
	imports := make([]string, 0, len(file.Imports))
	for _, importSpec := range file.Imports {
		if importSpec.Path == nil {
			continue
		}
		imports = append(imports, strings.Trim(importSpec.Path.Value, `"`))
	}
	sort.Strings(imports)
	return imports
}

func goSyntaxConstructCounts(counts map[string]int, omitFiles bool) []goSyntaxSelectionConstruct {
	constructs := make([]goSyntaxSelectionConstruct, 0, len(counts))
	for construct, count := range counts {
		if count == 0 {
			continue
		}
		entry := goSyntaxSelectionConstruct{
			Construct: construct,
			Count:     count,
		}
		if !omitFiles {
			entry.FileCount = 0
		}
		constructs = append(constructs, entry)
	}
	sort.Slice(constructs, func(i, j int) bool {
		return constructs[i].Construct < constructs[j].Construct
	})
	return constructs
}

func selectGoSyntaxCandidates(candidates []goSyntaxSelectionCandidate, config goSyntaxSelectionConfig) ([]goSyntaxSelectionCandidate, int64) {
	remaining := append([]goSyntaxSelectionCandidate(nil), candidates...)
	selected := make([]goSyntaxSelectionCandidate, 0, config.MaxFiles)
	coveredConstructs := map[string]bool{}
	packageCounts := map[string]int{}
	selectedBytes := int64(0)

	for len(selected) < config.MaxFiles && len(remaining) > 0 {
		bestIndex := -1
		bestScore := -1
		for i, candidate := range remaining {
			if selectedBytes+candidate.File.Bytes > config.MaxTotalBytes {
				continue
			}
			score := scoreGoSyntaxCandidate(candidate, coveredConstructs, packageCounts)
			if score > bestScore || (score == bestScore && bestIndex >= 0 && candidate.ManifestPath < remaining[bestIndex].ManifestPath) {
				bestIndex = i
				bestScore = score
			}
		}
		if bestIndex < 0 {
			break
		}

		candidate := remaining[bestIndex]
		candidate.Selected = true
		candidate.SelectionOrder = len(selected) + 1
		candidate.SelectionScore = bestScore
		candidate.SelectionReason = selectionReason(candidate, coveredConstructs, packageCounts)
		selected = append(selected, candidate)
		selectedBytes += candidate.File.Bytes
		for _, construct := range candidate.UniqueConstructs {
			coveredConstructs[construct] = true
		}
		packageCounts[candidate.Package]++
		remaining = append(remaining[:bestIndex], remaining[bestIndex+1:]...)
	}
	return selected, selectedBytes
}

func scoreGoSyntaxCandidate(candidate goSyntaxSelectionCandidate, covered map[string]bool, packageCounts map[string]int) int {
	newConstructs := 0
	if covered == nil {
		newConstructs = len(candidate.UniqueConstructs)
	} else {
		for _, construct := range candidate.UniqueConstructs {
			if !covered[construct] {
				newConstructs++
			}
		}
	}
	packageBonus := 0
	if packageCounts == nil || packageCounts[candidate.Package] == 0 {
		packageBonus = 25
	}
	sizePenalty := int(candidate.File.Bytes / 2048)
	return newConstructs*100 + len(candidate.UniqueConstructs)*2 + packageBonus - sizePenalty
}

func selectionReason(candidate goSyntaxSelectionCandidate, covered map[string]bool, packageCounts map[string]int) string {
	var reasons []string
	newConstructs := 0
	for _, construct := range candidate.UniqueConstructs {
		if !covered[construct] {
			newConstructs++
		}
	}
	if newConstructs > 0 {
		reasons = append(reasons, fmt.Sprintf("%d previously uncovered syntax constructs", newConstructs))
	}
	if packageCounts[candidate.Package] == 0 {
		reasons = append(reasons, "new package")
	}
	if len(reasons) == 0 {
		reasons = append(reasons, "fits remaining bounded corpus budget")
	}
	return strings.Join(reasons, "; ")
}

func summarizeGoSyntaxPackages(candidates []goSyntaxSelectionCandidate) []goSyntaxSelectionPackageSummary {
	byPackage := map[string]*goSyntaxSelectionPackageSummary{}
	for _, candidate := range candidates {
		summary := byPackage[candidate.Package]
		if summary == nil {
			summary = &goSyntaxSelectionPackageSummary{Package: candidate.Package}
			byPackage[candidate.Package] = summary
		}
		summary.FileCount++
		summary.Bytes += candidate.File.Bytes
		summary.Files = append(summary.Files, candidate.ManifestPath)
	}
	packages := make([]goSyntaxSelectionPackageSummary, 0, len(byPackage))
	for _, summary := range byPackage {
		sort.Strings(summary.Files)
		packages = append(packages, *summary)
	}
	sort.Slice(packages, func(i, j int) bool {
		return packages[i].Package < packages[j].Package
	})
	return packages
}

func summarizeGoSyntaxConstructs(candidates []goSyntaxSelectionCandidate) []goSyntaxSelectionConstruct {
	type accumulator struct {
		count int
		files map[string]bool
	}
	byConstruct := map[string]*accumulator{}
	for _, candidate := range candidates {
		for _, construct := range candidate.Constructs {
			acc := byConstruct[construct.Construct]
			if acc == nil {
				acc = &accumulator{files: map[string]bool{}}
				byConstruct[construct.Construct] = acc
			}
			acc.count += construct.Count
			acc.files[candidate.ManifestPath] = true
		}
	}
	constructs := make([]goSyntaxSelectionConstruct, 0, len(byConstruct))
	for construct, acc := range byConstruct {
		files := make([]string, 0, len(acc.files))
		for file := range acc.files {
			files = append(files, file)
		}
		sort.Strings(files)
		constructs = append(constructs, goSyntaxSelectionConstruct{
			Construct: construct,
			Count:     acc.count,
			FileCount: len(files),
			Files:     files,
		})
	}
	sort.Slice(constructs, func(i, j int) bool {
		return constructs[i].Construct < constructs[j].Construct
	})
	return constructs
}

func candidatePaths(candidates []goSyntaxSelectionCandidate) []string {
	paths := make([]string, 0, len(candidates))
	for _, candidate := range candidates {
		paths = append(paths, candidate.ManifestPath)
	}
	sort.Strings(paths)
	return paths
}

func goSyntaxSourceURI(prefix, manifestPath string) string {
	prefix = strings.TrimRight(prefix, "/")
	if prefix == "" {
		return ""
	}
	return prefix + "/" + manifestPath
}

func goSyntaxShapeHash(packageName string, constructs []string) string {
	parts := append([]string{"package=" + packageName}, constructs...)
	return hashGoSyntaxSelectionList(parts)
}

func hashGoSyntaxSelectionList(values []string) string {
	normalized := append([]string(nil), values...)
	sort.Strings(normalized)
	return hashGoSyntaxSelectionBytes([]byte(strings.Join(normalized, "\n")))
}

func hashGoSyntaxSelectionBytes(data []byte) string {
	digest := sha256.Sum256(data)
	return hex.EncodeToString(digest[:])
}

func writeGoSyntaxSelectionJSON(path string, stdout io.Writer, report *goSyntaxSelectionReport) error {
	var output io.Writer = stdout
	var file *os.File
	if path != "" {
		if err := os.MkdirAll(filepath.Dir(path), 0o755); err != nil {
			return fmt.Errorf("create output directory for %s: %w", path, err)
		}
		var err error
		file, err = os.Create(path)
		if err != nil {
			return fmt.Errorf("create %s: %w", path, err)
		}
		defer file.Close()
		output = file
	}
	encoder := json.NewEncoder(output)
	encoder.SetIndent("", "  ")
	if err := encoder.Encode(report); err != nil {
		return fmt.Errorf("write JSON selection report: %w", err)
	}
	return nil
}
