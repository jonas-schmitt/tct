package main

import "testing"

func TestGoAstSchemaInventoryBuildsPinnedLoweringSurface(t *testing.T) {
	report, err := buildGoAstSchemaInventory()
	if err != nil {
		t.Fatalf("build inventory: %v", err)
	}
	if err := validateGoAstSchemaInventory(report); err != nil {
		t.Fatalf("validate inventory: %v", err)
	}

	if report.Go.Version != "go1.24.3" {
		t.Fatalf("unexpected Go version %s", report.Go.Version)
	}
	if !goAstSchemaInventoryHasRule(report, "File") {
		t.Fatalf("missing File GrammarIR rule")
	}
	if !goAstSchemaInventoryHasRule(report, "FuncDecl") {
		t.Fatalf("missing FuncDecl GrammarIR rule")
	}
	if !goAstSchemaInventoryHasFiniteDomain(report, "go_token_declaration_tokens") {
		t.Fatalf("missing declaration-token finite domain")
	}
	if !goAstSchemaInventoryHasFiniteDomain(report, "go_ast_channel_directions") {
		t.Fatalf("missing channel-direction finite domain")
	}
	if !goAstSchemaInventoryHasFiniteDomain(report, "go_syntax_presence_call_expr_ellipsis") {
		t.Fatalf("missing call-expression ellipsis presence finite domain")
	}
	if !goAstSchemaInventoryHasFiniteDomain(report, "go_syntax_presence_func_type_func") {
		t.Fatalf("missing function-type func presence finite domain")
	}
	for _, field := range []string{"Lparen", "Rparen"} {
		assertGoAstSchemaPresenceField(t, report, "GenDecl", field, "go_syntax_presence_gen_decl_group")
	}
	assertGoAstSchemaPresenceField(t, report, "TypeSpec", "Assign", "go_syntax_presence_type_spec_alias")
	assertGoAstSchemaPresenceField(t, report, "CallExpr", "Ellipsis", "go_syntax_presence_call_expr_ellipsis")
	assertGoAstSchemaPresenceField(t, report, "FuncType", "Func", "go_syntax_presence_func_type_func")
	for _, expected := range []struct {
		node   string
		field  string
		policy string
	}{
		{node: "ArrayType", field: "Lbrack", policy: goAstSchemaPositionPolicyFixedSyntaxToken},
		{node: "BlockStmt", field: "Lbrace", policy: goAstSchemaPositionPolicyFixedSyntaxToken},
		{node: "BranchStmt", field: "TokPos", policy: goAstSchemaPositionPolicyFiniteDomainToken},
		{node: "CaseClause", field: "Case", policy: goAstSchemaPositionPolicySyntaxPresence},
		{node: "FieldList", field: "Opening", policy: goAstSchemaPositionPolicySyntaxPresence},
		{node: "Ident", field: "NamePos", policy: goAstSchemaPositionPolicyParserPositionSidecar},
		{node: "ImportSpec", field: "EndPos", policy: goAstSchemaPositionPolicyParserPositionSidecar},
		{node: "ParenExpr", field: "Lparen", policy: goAstSchemaPositionPolicyFixedSyntaxToken},
		{node: "UnaryExpr", field: "OpPos", policy: goAstSchemaPositionPolicyFiniteDomainToken},
		{node: "BadExpr", field: "From", policy: goAstSchemaPositionPolicyBadNodeSourceRange},
		{node: "Comment", field: "Slash", policy: goAstSchemaPositionPolicyCommentTriviaSidecar},
	} {
		assertGoAstSchemaPositionPolicy(t, report, expected.node, expected.field, expected.policy)
	}
	assertGoAstSchemaNoGenericPositionPolicy(t, report)
	if len(report.LoweringSamples) == 0 {
		t.Fatalf("expected lowering samples")
	}
	if len(report.StablePromotionBlockers) == 0 {
		t.Fatalf("stable-promotion blockers must stay explicit")
	}
}

func goAstSchemaInventoryHasRule(report *goAstSchemaInventoryReport, name string) bool {
	for _, rule := range report.GrammarIRRules {
		if rule.Name == name {
			return true
		}
	}
	return false
}

func goAstSchemaInventoryHasFiniteDomain(report *goAstSchemaInventoryReport, name string) bool {
	for _, domain := range report.FiniteDomains {
		if domain.Name == name {
			return true
		}
	}
	return false
}

func assertGoAstSchemaPresenceField(t *testing.T, report *goAstSchemaInventoryReport, ruleName string, fieldName string, domain string) {
	t.Helper()
	for _, rule := range report.GrammarIRRules {
		if rule.Name != ruleName {
			continue
		}
		for _, field := range rule.Fields {
			if field.Name != fieldName {
				continue
			}
			if field.Required {
				t.Fatalf("%s.%s should be optional syntax presence", ruleName, fieldName)
			}
			if field.Multiple {
				t.Fatalf("%s.%s should not be repeated", ruleName, fieldName)
			}
			if field.Policy != "syntax_presence_decision" {
				t.Fatalf("%s.%s policy = %s, want syntax_presence_decision", ruleName, fieldName, field.Policy)
			}
			if field.Domain != domain {
				t.Fatalf("%s.%s domain = %s, want %s", ruleName, fieldName, field.Domain, domain)
			}
			if len(field.Alternatives) != 2 {
				t.Fatalf("%s.%s alternatives = %d, want 2", ruleName, fieldName, len(field.Alternatives))
			}
			if field.Alternatives[0].Terminal != "boolean_payload" || field.Alternatives[1].Terminal != "boolean_payload" {
				t.Fatalf("%s.%s should use boolean payload alternatives", ruleName, fieldName)
			}
			return
		}
		t.Fatalf("missing %s.%s GrammarIR field", ruleName, fieldName)
	}
	t.Fatalf("missing %s GrammarIR rule", ruleName)
}

func assertGoAstSchemaPositionPolicy(t *testing.T, report *goAstSchemaInventoryReport, nodeName string, fieldName string, policy string) {
	t.Helper()
	for _, node := range report.NodeStructs {
		if node.Name != nodeName {
			continue
		}
		for _, field := range node.PositionFields {
			if field.Name != fieldName {
				continue
			}
			if field.Policy != policy {
				t.Fatalf("%s.%s policy = %s, want %s", nodeName, fieldName, field.Policy, policy)
			}
			return
		}
		t.Fatalf("missing position field %s.%s", nodeName, fieldName)
	}
	t.Fatalf("missing node %s", nodeName)
}

func assertGoAstSchemaNoGenericPositionPolicy(t *testing.T, report *goAstSchemaInventoryReport) {
	t.Helper()
	for _, node := range report.NodeStructs {
		for _, field := range node.PositionFields {
			if field.Policy == "source_position_or_delimiter_presence_sidecar" {
				t.Fatalf("%s.%s still uses the generic position sidecar policy", node.Name, field.Name)
			}
		}
	}
}
