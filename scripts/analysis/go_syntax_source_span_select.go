// go_syntax_source_span_select ranks parser-backed source spans inside one Go
// source file for a bounded generated-codec route tranche.
package main

import (
	"bytes"
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
	goSyntaxSpanSelectionArtifactKind = "tct_go_syntax_source_span_selection_v1"
	goSyntaxSpanSelectionVersion      = 1
	goSyntaxSpanSelectionTool         = "go_syntax_source_span_select"
	goSyntaxSpanNoStableClaim         = "parser-backed source-span selection evidence only; no generated-codec route, source-validity, arena IR, or stable backend promotion"
)

var goSyntaxSpanBlockedClaims = []string{
	"no_broad_corpus_claim",
	"no_source_validity_by_construction_claim",
	"no_type_correctness_claim",
	"no_model_quality_claim",
	"no_paper_facing_claim",
	"no_generated_hot_path_claim",
	"no_stable_backend_claim",
	"no_arena_ir_promotion",
}

var defaultGoSyntaxSpanTargetConstructs = []string{
	"channel_type",
	"comm_clause",
	"defer_statement",
	"ellipsis",
	"function_literal",
	"go_statement",
	"interface_type",
	"map_type",
	"range_statement",
	"select_statement",
	"send_statement",
	"type_assertion_expression",
}

type goSyntaxSpanSelectionConfig struct {
	SourcePath       string
	ManifestPath     string
	Output           string
	ValidateReport   string
	MaxSpanBytes     int
	TargetConstructs []string
	SelectionNote    string
}

type goSyntaxSpanSelectionReport struct {
	ArtifactKind      string                           `json:"artifact_kind"`
	Version           int                              `json:"version"`
	Tool              string                           `json:"tool"`
	GoVersion         string                           `json:"go_version"`
	Status            string                           `json:"status"`
	Source            goSyntaxSpanSource               `json:"source"`
	SelectionPolicy   goSyntaxSpanSelectionPolicy      `json:"selection_policy"`
	CandidateCount    int                              `json:"candidate_count"`
	SelectedCandidate *goSyntaxSpanSelectionCandidate  `json:"selected_candidate,omitempty"`
	Candidates        []goSyntaxSpanSelectionCandidate `json:"candidates"`
	ClaimBoundaries   goSyntaxSpanClaimBoundaries      `json:"claim_boundaries"`
}

type goSyntaxSpanSource struct {
	Path         string `json:"path"`
	ManifestPath string `json:"manifest_path"`
	SHA256       string `json:"sha256"`
	Bytes        int    `json:"bytes"`
	Package      string `json:"package"`
}

type goSyntaxSpanSelectionPolicy struct {
	MaxSpanBytes     int      `json:"max_span_bytes"`
	TargetConstructs []string `json:"target_constructs"`
	ScorePolicy      string   `json:"score_policy"`
	SelectionNote    string   `json:"selection_note"`
}

type goSyntaxSpanSelectionCandidate struct {
	SpanID           string                  `json:"span_id"`
	DeclKind         string                  `json:"decl_kind"`
	Name             string                  `json:"name"`
	ByteStart        int                     `json:"byte_start"`
	ByteEnd          int                     `json:"byte_end"`
	LineStart        int                     `json:"line_start"`
	LineEnd          int                     `json:"line_end"`
	Bytes            int                     `json:"bytes"`
	SHA256           string                  `json:"sha256"`
	UniqueConstructs []string                `json:"unique_constructs"`
	Constructs       []goSyntaxSpanConstruct `json:"constructs"`
	TargetConstructs []string                `json:"target_constructs"`
	SelectionScore   int                     `json:"selection_score"`
	SelectionOrder   int                     `json:"selection_order,omitempty"`
	Selected         bool                    `json:"selected"`
	SelectionReason  string                  `json:"selection_reason"`
}

type goSyntaxSpanConstruct struct {
	Construct string `json:"construct"`
	Count     int    `json:"count"`
}

type goSyntaxSpanClaimBoundaries struct {
	Boundary              string   `json:"boundary"`
	NoStableClaim         string   `json:"no_stable_claim"`
	BlockedClaims         []string `json:"blocked_claims"`
	ArenaIRPromotion      bool     `json:"arena_ir_promotion_allowed"`
	GeneratedCodecChanged bool     `json:"generated_codec_changed"`
}

func main() {
	if err := runGoSyntaxSpanSelection(os.Args[1:], os.Stdout); err != nil {
		fmt.Fprintf(os.Stderr, "%v\n", err)
		os.Exit(1)
	}
}

func runGoSyntaxSpanSelection(args []string, stdout io.Writer) error {
	config := defaultGoSyntaxSpanSelectionConfig()
	targetConstructs := strings.Join(config.TargetConstructs, ",")
	flags := flag.NewFlagSet(goSyntaxSpanSelectionTool, flag.ContinueOnError)
	flags.SetOutput(stdout)
	flags.StringVar(&config.SourcePath, "source", config.SourcePath, "checked Go source file to inspect")
	flags.StringVar(&config.ManifestPath, "manifest-path", config.ManifestPath, "manifest path recorded for the checked source")
	flags.StringVar(&config.Output, "output", "", "write JSON source-span selection report to this path instead of stdout")
	flags.StringVar(&config.ValidateReport, "validate-report", "", "validate a checked JSON report against the current source-span selection")
	flags.IntVar(&config.MaxSpanBytes, "max-span-bytes", config.MaxSpanBytes, "maximum byte size for a selected source span")
	flags.StringVar(&targetConstructs, "target-constructs", targetConstructs, "comma-separated construct classes preferred by span scoring")
	flags.StringVar(&config.SelectionNote, "selection-note", config.SelectionNote, "selection note recorded in the report")
	if err := flags.Parse(args); err != nil {
		if errors.Is(err, flag.ErrHelp) {
			return nil
		}
		return err
	}
	config.TargetConstructs = splitGoSyntaxSpanCSV(targetConstructs)
	report, err := buildGoSyntaxSpanSelectionReport(config)
	if err != nil {
		return err
	}
	if config.ValidateReport != "" {
		return validateGoSyntaxSpanSelectionReport(config.ValidateReport, report)
	}
	return writeGoSyntaxSpanSelectionJSON(config.Output, stdout, report)
}

func defaultGoSyntaxSpanSelectionConfig() goSyntaxSpanSelectionConfig {
	return goSyntaxSpanSelectionConfig{
		MaxSpanBytes:     2500,
		TargetConstructs: append([]string{}, defaultGoSyntaxSpanTargetConstructs...),
		SelectionNote: "Parser-backed Go source-span selection evidence for a bounded " +
			"generated-codec report-production tranche; generated-codec implementation " +
			"and corpus packaging remain the next step.",
	}
}

func buildGoSyntaxSpanSelectionReport(
	config goSyntaxSpanSelectionConfig,
) (*goSyntaxSpanSelectionReport, error) {
	if strings.TrimSpace(config.SourcePath) == "" {
		return nil, errors.New("--source is required")
	}
	if strings.TrimSpace(config.ManifestPath) == "" {
		return nil, errors.New("--manifest-path is required")
	}
	if config.MaxSpanBytes <= 0 {
		return nil, errors.New("--max-span-bytes must be positive")
	}
	if len(config.TargetConstructs) == 0 {
		return nil, errors.New("--target-constructs must contain at least one construct")
	}
	source, err := os.ReadFile(config.SourcePath)
	if err != nil {
		return nil, fmt.Errorf("read source %s: %w", config.SourcePath, err)
	}
	fset := token.NewFileSet()
	parsed, err := parser.ParseFile(fset, config.SourcePath, source, parser.ParseComments)
	if err != nil {
		return nil, fmt.Errorf("parse source %s: %w", config.SourcePath, err)
	}
	candidates, err := goSyntaxSpanCandidates(source, fset, parsed, config.TargetConstructs)
	if err != nil {
		return nil, err
	}
	sort.Slice(candidates, func(left, right int) bool {
		if candidates[left].SelectionScore != candidates[right].SelectionScore {
			return candidates[left].SelectionScore > candidates[right].SelectionScore
		}
		if candidates[left].Bytes != candidates[right].Bytes {
			return candidates[left].Bytes < candidates[right].Bytes
		}
		return candidates[left].SpanID < candidates[right].SpanID
	})
	for i := range candidates {
		candidates[i].SelectionOrder = i + 1
	}
	var selected *goSyntaxSpanSelectionCandidate
	for i := range candidates {
		if candidates[i].Bytes > config.MaxSpanBytes {
			continue
		}
		candidates[i].Selected = true
		candidates[i].SelectionReason = fmt.Sprintf(
			"%d target constructs within %d byte source-span budget",
			len(candidates[i].TargetConstructs),
			config.MaxSpanBytes,
		)
		selected = &candidates[i]
		break
	}
	status := "ready"
	if selected == nil {
		status = "blocked"
	}
	return &goSyntaxSpanSelectionReport{
		ArtifactKind: goSyntaxSpanSelectionArtifactKind,
		Version:      goSyntaxSpanSelectionVersion,
		Tool:         goSyntaxSpanSelectionTool,
		GoVersion:    runtime.Version(),
		Status:       status,
		Source: goSyntaxSpanSource{
			Path:         filepath.ToSlash(config.SourcePath),
			ManifestPath: filepath.ToSlash(config.ManifestPath),
			SHA256:       goSyntaxSpanHash(source),
			Bytes:        len(source),
			Package:      parsed.Name.Name,
		},
		SelectionPolicy: goSyntaxSpanSelectionPolicy{
			MaxSpanBytes:     config.MaxSpanBytes,
			TargetConstructs: append([]string{}, config.TargetConstructs...),
			ScorePolicy:      "rank by covered target constructs, then total syntax constructs, then smaller byte span",
			SelectionNote:    config.SelectionNote,
		},
		CandidateCount:    len(candidates),
		SelectedCandidate: selected,
		Candidates:        candidates,
		ClaimBoundaries: goSyntaxSpanClaimBoundaries{
			Boundary:              "parser-backed metadata -> GrammarIR -> StructuralModel -> token layout -> generated codec",
			NoStableClaim:         goSyntaxSpanNoStableClaim,
			BlockedClaims:         append([]string{}, goSyntaxSpanBlockedClaims...),
			ArenaIRPromotion:      false,
			GeneratedCodecChanged: false,
		},
	}, nil
}

func goSyntaxSpanCandidates(
	source []byte,
	fset *token.FileSet,
	file *ast.File,
	targetConstructs []string,
) ([]goSyntaxSpanSelectionCandidate, error) {
	candidates := make([]goSyntaxSpanSelectionCandidate, 0, len(file.Decls))
	for _, decl := range file.Decls {
		startPos, endPos := goSyntaxSpanDeclRange(decl)
		start := fset.Position(startPos)
		end := fset.Position(endPos)
		if start.Offset < 0 || end.Offset > len(source) || start.Offset >= end.Offset {
			return nil, fmt.Errorf("invalid source span offsets for %s", goSyntaxSpanDeclName(decl))
		}
		spanBytes := source[start.Offset:end.Offset]
		counts := collectGoSyntaxSpanConstructs(decl)
		constructs := goSyntaxSpanConstructCounts(counts)
		uniqueConstructs := make([]string, 0, len(constructs))
		for _, construct := range constructs {
			uniqueConstructs = append(uniqueConstructs, construct.Construct)
		}
		matchedTargets := goSyntaxSpanMatchedTargets(counts, targetConstructs)
		name := goSyntaxSpanDeclName(decl)
		candidates = append(candidates, goSyntaxSpanSelectionCandidate{
			SpanID:           goSyntaxSpanID(decl, name),
			DeclKind:         goSyntaxSpanDeclKind(decl),
			Name:             name,
			ByteStart:        start.Offset,
			ByteEnd:          end.Offset,
			LineStart:        start.Line,
			LineEnd:          end.Line,
			Bytes:            len(spanBytes),
			SHA256:           goSyntaxSpanHash(spanBytes),
			UniqueConstructs: uniqueConstructs,
			Constructs:       constructs,
			TargetConstructs: matchedTargets,
			SelectionScore:   goSyntaxSpanScore(matchedTargets, constructs, len(spanBytes)),
		})
	}
	return candidates, nil
}

func goSyntaxSpanDeclRange(decl ast.Decl) (token.Pos, token.Pos) {
	start := decl.Pos()
	switch typed := decl.(type) {
	case *ast.FuncDecl:
		if typed.Doc != nil {
			start = typed.Doc.Pos()
		}
	case *ast.GenDecl:
		if typed.Doc != nil {
			start = typed.Doc.Pos()
		}
	}
	return start, decl.End()
}

func collectGoSyntaxSpanConstructs(node ast.Node) map[string]int {
	counts := map[string]int{}
	ast.Inspect(node, func(current ast.Node) bool {
		switch n := current.(type) {
		case *ast.GenDecl:
			switch n.Tok {
			case token.IMPORT:
				counts["import_declaration"]++
			case token.CONST:
				counts["const_declaration"]++
			case token.TYPE:
				counts["type_declaration"]++
			case token.VAR:
				counts["var_declaration"]++
			}
		case *ast.FuncDecl:
			if n.Recv != nil {
				counts["method_declaration"]++
			} else {
				counts["function_declaration"]++
			}
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
		case *ast.IfStmt:
			counts["if_statement"]++
		case *ast.ForStmt:
			counts["for_statement"]++
		case *ast.RangeStmt:
			counts["range_statement"]++
		case *ast.DeferStmt:
			counts["defer_statement"]++
		case *ast.GoStmt:
			counts["go_statement"]++
		case *ast.SendStmt:
			counts["send_statement"]++
		case *ast.SelectStmt:
			counts["select_statement"]++
		case *ast.CommClause:
			counts["comm_clause"]++
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
		case *ast.Ellipsis:
			counts["ellipsis"]++
		}
		return true
	})
	return counts
}

func goSyntaxSpanDeclName(decl ast.Decl) string {
	switch typed := decl.(type) {
	case *ast.FuncDecl:
		if typed.Recv == nil || len(typed.Recv.List) == 0 {
			return typed.Name.Name
		}
		return goSyntaxSpanReceiverName(typed.Recv.List[0].Type) + "." + typed.Name.Name
	case *ast.GenDecl:
		names := make([]string, 0, len(typed.Specs))
		for _, spec := range typed.Specs {
			switch spec := spec.(type) {
			case *ast.TypeSpec:
				names = append(names, spec.Name.Name)
			case *ast.ValueSpec:
				for _, name := range spec.Names {
					names = append(names, name.Name)
				}
			}
		}
		if len(names) == 0 {
			return strings.ToLower(typed.Tok.String())
		}
		return strings.ToLower(typed.Tok.String()) + " " + strings.Join(names, ",")
	default:
		return "decl"
	}
}

func goSyntaxSpanReceiverName(expr ast.Expr) string {
	switch typed := expr.(type) {
	case *ast.Ident:
		return typed.Name
	case *ast.StarExpr:
		return goSyntaxSpanReceiverName(typed.X)
	default:
		return "receiver"
	}
}

func goSyntaxSpanDeclKind(decl ast.Decl) string {
	switch typed := decl.(type) {
	case *ast.FuncDecl:
		if typed.Recv == nil {
			return "function_declaration"
		}
		return "method_declaration"
	case *ast.GenDecl:
		return strings.ToLower(typed.Tok.String()) + "_declaration"
	default:
		return "declaration"
	}
}

func goSyntaxSpanID(decl ast.Decl, name string) string {
	normalized := strings.ToLower(name)
	replacer := strings.NewReplacer(" ", "_", ",", "_", ".", "_", "*", "", "-", "_", "/", "_")
	return goSyntaxSpanDeclKind(decl) + ":" + strings.Trim(replacer.Replace(normalized), "_")
}

func goSyntaxSpanConstructCounts(counts map[string]int) []goSyntaxSpanConstruct {
	constructs := make([]goSyntaxSpanConstruct, 0, len(counts))
	for construct, count := range counts {
		if count == 0 {
			continue
		}
		constructs = append(constructs, goSyntaxSpanConstruct{Construct: construct, Count: count})
	}
	sort.Slice(constructs, func(left, right int) bool {
		return constructs[left].Construct < constructs[right].Construct
	})
	return constructs
}

func goSyntaxSpanMatchedTargets(counts map[string]int, targetConstructs []string) []string {
	matched := make([]string, 0, len(targetConstructs))
	for _, target := range targetConstructs {
		if counts[target] > 0 {
			matched = append(matched, target)
		}
	}
	sort.Strings(matched)
	return matched
}

func goSyntaxSpanScore(
	matchedTargets []string,
	constructs []goSyntaxSpanConstruct,
	spanBytes int,
) int {
	totalConstructs := 0
	for _, construct := range constructs {
		totalConstructs += construct.Count
	}
	return len(matchedTargets)*10_000 + totalConstructs*10 - spanBytes/100
}

func splitGoSyntaxSpanCSV(value string) []string {
	parts := strings.Split(value, ",")
	result := make([]string, 0, len(parts))
	seen := map[string]bool{}
	for _, part := range parts {
		normalized := strings.TrimSpace(part)
		if normalized == "" || seen[normalized] {
			continue
		}
		seen[normalized] = true
		result = append(result, normalized)
	}
	sort.Strings(result)
	return result
}

func writeGoSyntaxSpanSelectionJSON(
	path string,
	stdout io.Writer,
	report *goSyntaxSpanSelectionReport,
) error {
	data, err := json.MarshalIndent(report, "", "  ")
	if err != nil {
		return err
	}
	data = append(data, '\n')
	if path == "" {
		_, err = stdout.Write(data)
		return err
	}
	if err := os.MkdirAll(filepath.Dir(path), 0o755); err != nil {
		return err
	}
	return os.WriteFile(path, data, 0o644)
}

func validateGoSyntaxSpanSelectionReport(
	path string,
	actual *goSyntaxSpanSelectionReport,
) error {
	expectedData, err := os.ReadFile(path)
	if err != nil {
		return fmt.Errorf("read validate-report %s: %w", path, err)
	}
	var expected goSyntaxSpanSelectionReport
	if err := json.Unmarshal(expectedData, &expected); err != nil {
		return fmt.Errorf("parse validate-report %s: %w", path, err)
	}
	expectedJSON, err := json.Marshal(expected)
	if err != nil {
		return err
	}
	actualJSON, err := json.Marshal(actual)
	if err != nil {
		return err
	}
	if !bytes.Equal(expectedJSON, actualJSON) {
		return fmt.Errorf("source-span selection report %s does not match current source evidence", path)
	}
	return nil
}

func goSyntaxSpanHash(data []byte) string {
	sum := sha256.Sum256(data)
	return hex.EncodeToString(sum[:])
}
