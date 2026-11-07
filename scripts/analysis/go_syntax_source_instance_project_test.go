package main

import (
	"bytes"
	"go/ast"
	"go/format"
	"go/parser"
	"go/token"
	"os"
	"path/filepath"
	"strings"
	"testing"
)

func TestSourceInstanceProjectionProjectsSyntheticGoAst(t *testing.T) {
	dir := t.TempDir()
	sourcePath := filepath.Join(dir, "main.go")
	metadataPath := filepath.Join(dir, "syntax-metadata-v0.json")
	projectionPath := filepath.Join(dir, "projection.json")
	reportPath := filepath.Join(dir, "report.json")
	source := []byte("package main\n\nconst answer = 42\n")
	if err := os.WriteFile(sourcePath, source, 0o644); err != nil {
		t.Fatal(err)
	}
	metadata := syntheticSourceInstanceMetadata(sourcePath, sha256Hex(source))
	if err := writeJSON(metadataPath, metadata); err != nil {
		t.Fatal(err)
	}

	options := sourceInstanceOptions{
		sourcePath:         sourcePath,
		metadataPath:       metadataPath,
		manifestPath:       "main.go",
		candidateGrammarID: "go_synthetic_source_file_syntax_v0",
		projectionOutput:   projectionPath,
		reportOutput:       reportPath,
	}
	projection, report, err := buildSourceInstanceProjectionAndReport(options)
	if err != nil {
		t.Fatal(err)
	}

	if projection.Root.NodeKind != "File" {
		t.Fatalf("root node kind = %q, want File", projection.Root.NodeKind)
	}
	if projection.Stats.NodeCount != 6 {
		t.Fatalf("node count = %d, want 6", projection.Stats.NodeCount)
	}
	if projection.Stats.TerminalPayloadCount != 5 {
		t.Fatalf("terminal payload count = %d, want 5", projection.Stats.TerminalPayloadCount)
	}
	if !report.CanonicalRender.MatchesSource {
		t.Fatal("canonical render should match gofmt-formatted synthetic source")
	}
	if !report.CurrentEvidence.SourceInstanceProjectionReady {
		t.Fatal("source instance projection should be ready")
	}
	if report.CurrentEvidence.GeneratedCodecReportReady {
		t.Fatal("generated-codec report must remain blocked")
	}
	if !strings.Contains(report.NextConcreteTask, "reusable generated-codec renderer/report machinery") {
		t.Fatalf("next task lost renderer handoff: %s", report.NextConcreteTask)
	}
	if len(report.CurrentBlockers) != 3 {
		t.Fatalf("blocker count = %d, want 3", len(report.CurrentBlockers))
	}
}

func TestSourceInstanceProjectionProjectsFileHeaderComments(t *testing.T) {
	dir := t.TempDir()
	projectionPath := filepath.Join(dir, "projection.json")
	reportPath := filepath.Join(dir, "report.json")
	options := sourceInstanceOptions{
		sourcePath:           fixturePath(t, "tests/fixtures/go_syntax_corpus/src/strings/compare.go"),
		metadataPath:         fixturePath(t, "tests/fixtures/go_syntax_metadata/strings_compare_source_file/syntax-metadata-v0.json"),
		metadataSmokeFixture: "strings-compare-source-file",
		manifestPath:         "strings/compare.go",
		candidateGrammarID:   "go_strings_compare_source_file_syntax_v0",
		projectionOutput:     projectionPath,
		reportOutput:         reportPath,
	}
	projection, report, err := buildSourceInstanceProjectionAndReport(options)
	if err != nil {
		t.Fatal(err)
	}

	var headerComments projectionField
	found := false
	for _, field := range projection.Root.Fields {
		if field.Name == "HeaderComments" {
			headerComments = field
			found = true
			break
		}
	}
	if !found {
		t.Fatal("projection root should include HeaderComments")
	}
	if len(headerComments.Values) != 1 {
		t.Fatalf("header comment groups = %d, want 1", len(headerComments.Values))
	}
	if headerComments.Values[0].NodeKind != "CommentGroup" {
		t.Fatalf("header comment node kind = %q, want CommentGroup", headerComments.Values[0].NodeKind)
	}
	if !report.CanonicalRender.MatchesSource {
		t.Fatal("canonical render should match gofmt-formatted strings/compare source")
	}
	commands := strings.Join(report.ValidationCommands, "\n")
	if !strings.Contains(commands, "--metadata-smoke-fixture strings-compare-source-file") {
		t.Fatalf("validation commands must preserve strings/compare metadata smoke fixture:\n%s", commands)
	}
}

func TestSourceInstanceProjectionProjectsBooleanPayload(t *testing.T) {
	dir := t.TempDir()
	projectionPath := filepath.Join(dir, "projection.json")
	reportPath := filepath.Join(dir, "report.json")
	options := sourceInstanceOptions{
		sourcePath:           fixturePath(t, "tests/fixtures/go_syntax_corpus/src/slice_expression_three_index/main.go"),
		metadataPath:         fixturePath(t, "tests/fixtures/go_syntax_metadata/slice_expression_three_index/syntax-metadata-v0.json"),
		metadataSmokeFixture: "slice-expression-three-index",
		manifestPath:         "slice_expression_three_index/main.go",
		candidateGrammarID:   "go_slice_expression_three_index_syntax_v0",
		projectionOutput:     projectionPath,
		reportOutput:         reportPath,
		routeRegistered:      true,
	}
	projection, report, err := buildSourceInstanceProjectionAndReport(options)
	if err != nil {
		t.Fatal(err)
	}

	if got := projection.Stats.TerminalPayloadsByKind["boolean_payload"]; got != 1 {
		t.Fatalf("boolean payload count = %d, want 1", got)
	}
	if !report.CanonicalRender.MatchesSource {
		t.Fatal("canonical render should match gofmt-formatted three-index slice source")
	}
	if len(report.CurrentBlockers) != 2 {
		t.Fatalf("registered route blocker count = %d, want 2", len(report.CurrentBlockers))
	}
	commands := strings.Join(report.ValidationCommands, "\n")
	if !strings.Contains(commands, "--metadata-smoke-fixture slice-expression-three-index") {
		t.Fatalf("validation commands must preserve three-index metadata smoke fixture:\n%s", commands)
	}
}

func TestEmptyClauseBodyCommentAnchorsStayOnClause(t *testing.T) {
	source := []byte("package p\n\nfunc f() {\n\tselect {\n\tcase done <- call:\n\t\t// ok\n\tcase <-other:\n\t\treturn\n\t}\n}\n")
	fileSet := token.NewFileSet()
	file, err := parser.ParseFile(fileSet, "source.go", source, parser.ParseComments)
	if err != nil {
		t.Fatal(err)
	}
	attached := attachedCommentGroupPositions(file)
	anchors, positions := emptyClauseBodyCommentAnchors(file, fileSet, file.Comments, attached)

	var selectBody *ast.BlockStmt
	var emptyClause *ast.CommClause
	ast.Inspect(file, func(node ast.Node) bool {
		switch typed := node.(type) {
		case *ast.SelectStmt:
			selectBody = typed.Body
		case *ast.CommClause:
			if len(typed.Body) == 0 {
				emptyClause = typed
			}
		}
		return true
	})
	if selectBody == nil || emptyClause == nil {
		t.Fatal("expected test source to contain a select body with one empty CommClause")
	}
	if got := anchors[emptyClause.Colon]; len(got) != 1 || got[0] != 0 {
		t.Fatalf("empty CommClause anchors = %#v, want []int{0}", got)
	}
	if _, ok := positions[file.Comments[0].Pos()]; !ok {
		t.Fatalf("empty clause comment position was not classified as clause-owned")
	}
	parentIndexes := blockStmtCommentBeforeIndexes(selectBody, fileSet, file.Comments, attached, positions)
	if len(parentIndexes) != 0 {
		t.Fatalf("select parent comment indexes = %#v, want none", parentIndexes)
	}
	parentBlankIndexes := blockStmtBlankLineBeforeIndexes(selectBody, fileSet, file.Comments, attached, positions)
	if len(parentBlankIndexes) != 0 {
		t.Fatalf("select parent blank-line indexes = %#v, want none", parentBlankIndexes)
	}
}

func TestCompositeLiteralLineBreakBeforeIndexes(t *testing.T) {
	source := []byte("package p\n\nfunc f() {\n\tcompact := &Client{codec: codec, pending: make(map[uint64]*Call)}\n\tmulti := &Client{\n\t\tcodec:   codec,\n\t\tpending: make(map[uint64]*Call),\n\t}\n\t_ = compact\n\t_ = multi\n}\n")
	fileSet := token.NewFileSet()
	file, err := parser.ParseFile(fileSet, "source.go", source, parser.ParseComments)
	if err != nil {
		t.Fatal(err)
	}

	literals := make([]*ast.CompositeLit, 0)
	ast.Inspect(file, func(node ast.Node) bool {
		if literal, ok := node.(*ast.CompositeLit); ok {
			literals = append(literals, literal)
		}
		return true
	})
	if len(literals) < 2 {
		t.Fatalf("literal count = %d, want at least 2", len(literals))
	}

	if got := compositeLitLineBreakBeforeIndexes(literals[0], fileSet); len(got) != 0 {
		t.Fatalf("compact composite literal line breaks = %#v, want none", got)
	}
	got := compositeLitLineBreakBeforeIndexes(literals[1], fileSet)
	if len(got) != 2 || got[0] != 0 || got[1] != 1 {
		t.Fatalf("multi-line composite literal line breaks = %#v, want []int{0, 1}", got)
	}
}

func TestCallExprLineBreakBeforeIndexes(t *testing.T) {
	source := []byte("package p\n\nfunc f() {\n\tcompact := fmt.Errorf(\"reading meta-data file %s: %v\", p.MetaFile, err)\n\tmulti := fmt.Errorf(\"reading meta-data file %s: %v\",\n\t\tp.MetaFile, err)\n\t_ = compact\n\t_ = multi\n}\n")
	fileSet := token.NewFileSet()
	file, err := parser.ParseFile(fileSet, "source.go", source, parser.ParseComments)
	if err != nil {
		t.Fatal(err)
	}

	calls := make([]*ast.CallExpr, 0)
	ast.Inspect(file, func(node ast.Node) bool {
		if call, ok := node.(*ast.CallExpr); ok {
			calls = append(calls, call)
		}
		return true
	})
	if len(calls) < 2 {
		t.Fatalf("call count = %d, want at least 2", len(calls))
	}

	if got := callExprLineBreakBeforeIndexes(calls[0], fileSet); len(got) != 0 {
		t.Fatalf("compact call line breaks = %#v, want none", got)
	}
	got := callExprLineBreakBeforeIndexes(calls[1], fileSet)
	if len(got) != 1 || got[0] != 1 {
		t.Fatalf("multi-line call line breaks = %#v, want []int{1}", got)
	}
}

func fixturePath(t *testing.T, path string) string {
	t.Helper()
	if _, err := os.Stat(path); err == nil {
		return path
	}
	parentPath := filepath.Join("..", "..", path)
	if _, err := os.Stat(parentPath); err == nil {
		return parentPath
	}
	t.Fatalf("fixture %s not found from test working directory", path)
	return ""
}

func TestSourceInstanceReportValidationDetectsDrift(t *testing.T) {
	dir := t.TempDir()
	sourcePath := filepath.Join(dir, "main.go")
	metadataPath := filepath.Join(dir, "syntax-metadata-v0.json")
	projectionPath := filepath.Join(dir, "projection.json")
	reportPath := filepath.Join(dir, "report.json")
	source := []byte("package main\n\nconst answer = 42\n")
	if err := os.WriteFile(sourcePath, source, 0o644); err != nil {
		t.Fatal(err)
	}
	metadata := syntheticSourceInstanceMetadata(sourcePath, sha256Hex(source))
	if err := writeJSON(metadataPath, metadata); err != nil {
		t.Fatal(err)
	}

	options := sourceInstanceOptions{
		sourcePath:         sourcePath,
		metadataPath:       metadataPath,
		manifestPath:       "main.go",
		candidateGrammarID: "go_synthetic_source_file_syntax_v0",
		projectionOutput:   projectionPath,
		reportOutput:       reportPath,
	}
	_, report, err := buildSourceInstanceProjectionAndReport(options)
	if err != nil {
		t.Fatal(err)
	}
	if err := writeJSON(reportPath, report); err != nil {
		t.Fatal(err)
	}
	if err := validateSourceInstanceReport(reportPath, report); err != nil {
		t.Fatal(err)
	}

	report.Status = "stale"
	if err := writeJSON(reportPath, report); err != nil {
		t.Fatal(err)
	}
	_, expected, err := buildSourceInstanceProjectionAndReport(options)
	if err != nil {
		t.Fatal(err)
	}
	if err := validateSourceInstanceReport(reportPath, expected); err == nil {
		t.Fatal("expected validation drift error")
	}
}

func TestSourceInstanceProjectionRecordsCurrentReportIngestionManifest(t *testing.T) {
	dir := t.TempDir()
	sourcePath := filepath.Join(dir, "main.go")
	metadataPath := filepath.Join(dir, "syntax-metadata-v0.json")
	projectionPath := filepath.Join(dir, "projection.json")
	reportPath := filepath.Join(dir, "report.json")
	source := []byte("package main\n\nconst answer = 42\n")
	if err := os.WriteFile(sourcePath, source, 0o644); err != nil {
		t.Fatal(err)
	}
	metadata := syntheticSourceInstanceMetadata(sourcePath, sha256Hex(source))
	if err := writeJSON(metadataPath, metadata); err != nil {
		t.Fatal(err)
	}

	reportIngestionManifestPath := "tests/fixtures/go_syntax_corpus/training-report-ingestion-eleven-record-v1.json"
	options := sourceInstanceOptions{
		sourcePath:                  sourcePath,
		metadataPath:                metadataPath,
		metadataSmokeFixture:        "syscall-fs-js-source-file",
		manifestPath:                "syscall/fs_js.go",
		candidateGrammarID:          "go_syscall_fs_js_source_file_syntax_v0",
		reportIngestionManifestPath: reportIngestionManifestPath,
		projectionOutput:            projectionPath,
		reportOutput:                reportPath,
	}
	_, report, err := buildSourceInstanceProjectionAndReport(options)
	if err != nil {
		t.Fatal(err)
	}

	if report.ReportIngestionManifestPath != reportIngestionManifestPath {
		t.Fatalf("report-ingestion manifest path = %q, want %q", report.ReportIngestionManifestPath, reportIngestionManifestPath)
	}
	if report.CurrentBlockers[2].Path != reportIngestionManifestPath {
		t.Fatalf("report-ingestion blocker path = %q, want %q", report.CurrentBlockers[2].Path, reportIngestionManifestPath)
	}
	commands := strings.Join(report.ValidationCommands, "\n")
	if !strings.Contains(commands, "--metadata-smoke-fixture syscall-fs-js-source-file") {
		t.Fatalf("validation commands must preserve metadata smoke fixture:\n%s", commands)
	}
	if !strings.Contains(commands, "--report-ingestion-manifest "+reportIngestionManifestPath) {
		t.Fatalf("validation commands must preserve report-ingestion manifest:\n%s", commands)
	}
	if err := writeJSON(reportPath, report); err != nil {
		t.Fatal(err)
	}
	if err := validateSourceInstanceReport(reportPath, report); err != nil {
		t.Fatal(err)
	}
}

func TestSourceInstanceProjectionNormalizesNonGofmtSourceToCanonicalForm(t *testing.T) {
	dir := t.TempDir()
	sourcePath := filepath.Join(dir, "zdefaultcc_style.go")
	metadataPath := filepath.Join(dir, "syntax-metadata-v0.json")
	projectionPath := filepath.Join(dir, "projection.json")
	reportPath := filepath.Join(dir, "report.json")
	// Deliberately non-gofmt spacing (stdlib residue class: gofmt adds blank
	// line after package and spaces around =). Stay within synthetic GenDecl metadata.
	source := []byte("package main\nconst answer=42\n")
	if err := os.WriteFile(sourcePath, source, 0o644); err != nil {
		t.Fatal(err)
	}
	formatted, err := format.Source(source)
	if err != nil {
		t.Fatal(err)
	}
	if bytes.Equal(source, formatted) {
		t.Fatal("test fixture source must not already be gofmt-canonical")
	}
	// Metadata integrity pins the on-disk original; projection works on gofmt form.
	metadata := syntheticSourceInstanceMetadata(sourcePath, sha256Hex(source))
	if err := writeJSON(metadataPath, metadata); err != nil {
		t.Fatal(err)
	}

	options := sourceInstanceOptions{
		sourcePath:         sourcePath,
		metadataPath:       metadataPath,
		manifestPath:       "zdefaultcc_style.go",
		candidateGrammarID: "go_synthetic_source_file_syntax_v0",
		projectionOutput:   projectionPath,
		reportOutput:       reportPath,
	}
	projection, report, err := buildSourceInstanceProjectionAndReport(options)
	if err != nil {
		t.Fatal(err)
	}
	if projection.Root.NodeKind != "File" {
		t.Fatalf("root node kind = %q, want File", projection.Root.NodeKind)
	}
	if !report.CanonicalRender.MatchesSource {
		t.Fatal("gofmt-canonical normalize must make matches_source true")
	}
	wantHash := sha256Hex(formatted)
	if report.Source.SHA256 != wantHash {
		t.Fatalf("projection source sha256 = %s, want gofmt form %s", report.Source.SHA256, wantHash)
	}
	if report.CanonicalRender.CanonicalSHA256 != wantHash {
		t.Fatalf("canonical_render sha256 = %s, want gofmt form %s", report.CanonicalRender.CanonicalSHA256, wantHash)
	}
	if report.Source.Bytes != len(formatted) {
		t.Fatalf("projection source bytes = %d, want gofmt form %d", report.Source.Bytes, len(formatted))
	}
	if report.CanonicalRender.CanonicalByteCount != len(formatted) {
		t.Fatalf("canonical byte count = %d, want %d", report.CanonicalRender.CanonicalByteCount, len(formatted))
	}
	if report.Source.Path == sourcePath {
		t.Fatal("non-gofmt source must be rewritten to a gofmt-canonical working path")
	}
	working, err := os.ReadFile(report.Source.Path)
	if err != nil {
		t.Fatal(err)
	}
	if !bytes.Equal(working, formatted) {
		t.Fatal("gofmt-canonical working copy must match format.Source output")
	}
}
