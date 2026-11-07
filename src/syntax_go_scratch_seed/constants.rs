pub const GO_AST_INVENTORY_SCRATCH_VAR_DECL_RENDERED_SOURCE: &str = "package p\n\nvar x = 0\n";
pub const GO_AST_INVENTORY_SCRATCH_VAR_DECL_ROOT: &str = "GoSourceFile";
pub const GO_AST_INVENTORY_SCRATCH_VAR_DECL_SEED_ID: &str =
    "go_ast_inventory_scratch_var_decl_seed";
pub const GO_AST_INVENTORY_SCRATCH_VAR_DECL_GRAMMAR_CATEGORY: &str = "package_var_declaration";
pub const GO_AST_INVENTORY_SCRATCH_VAR_DECL_PACKAGE_NAME: &str = "p";
pub const GO_AST_INVENTORY_SCRATCH_VAR_DECL_IDENTIFIER: &str = "x";
pub const GO_AST_INVENTORY_SCRATCH_VAR_DECL_LITERAL_KIND: &str = "INT";
pub const GO_AST_INVENTORY_SCRATCH_VAR_DECL_LITERAL_VALUE: &str = "0";
pub const GO_AST_INVENTORY_SCRATCH_VAR_DECL_GO_VERSION: &str = "go1.24";
pub const GO_AST_INVENTORY_SCRATCH_VAR_DECL_CONTRACT: &str = "token-only GoSourceFile scratch seed selected stepwise from raw state-machine admissible masks";
pub const GO_AST_INVENTORY_SCRATCH_FUNCTION_RETURN_RENDERED_SOURCE: &str =
    "package p\n\nfunc f() int {\n\treturn 7\n}\n";
pub const GO_AST_INVENTORY_SCRATCH_FUNCTION_RETURN_ROOT: &str = "GoSourceFile";
pub const GO_AST_INVENTORY_SCRATCH_FUNCTION_RETURN_SEED_ID: &str =
    "go_ast_inventory_scratch_function_return_seed";
pub const GO_AST_INVENTORY_SCRATCH_FUNCTION_RETURN_GRAMMAR_CATEGORY: &str =
    "function_return_integer_literal";
pub const GO_AST_INVENTORY_SCRATCH_FUNCTION_RETURN_PACKAGE_NAME: &str = "p";
pub const GO_AST_INVENTORY_SCRATCH_FUNCTION_RETURN_IDENTIFIER: &str = "f";
pub const GO_AST_INVENTORY_SCRATCH_FUNCTION_RETURN_RESULT_TYPE: &str = "int";
pub const GO_AST_INVENTORY_SCRATCH_FUNCTION_RETURN_LITERAL_KIND: &str = "INT";
pub const GO_AST_INVENTORY_SCRATCH_FUNCTION_RETURN_LITERAL_VALUE: &str = "7";
pub const GO_AST_INVENTORY_SCRATCH_FUNCTION_RETURN_GO_VERSION: &str = "go1.24";
pub const GO_AST_INVENTORY_SCRATCH_FUNCTION_RETURN_CONTRACT: &str = "token-only GoSourceFile function-return scratch seed selected stepwise from raw state-machine admissible masks";
pub const GO_AST_INVENTORY_SCRATCH_FUNC_TYPE_RESULT_RENDERED_SOURCE: &str =
    "package p\n\nfunc f() func() {\n\treturn nil\n}\n";
pub const GO_AST_INVENTORY_SCRATCH_FUNC_TYPE_RESULT_ROOT: &str = "GoSourceFile";
pub const GO_AST_INVENTORY_SCRATCH_FUNC_TYPE_RESULT_SEED_ID: &str =
    "go_ast_inventory_scratch_func_type_result_seed";
pub const GO_AST_INVENTORY_SCRATCH_FUNC_TYPE_RESULT_GRAMMAR_CATEGORY: &str = "function_type_result";
pub const GO_AST_INVENTORY_SCRATCH_FUNC_TYPE_RESULT_PACKAGE_NAME: &str = "p";
pub const GO_AST_INVENTORY_SCRATCH_FUNC_TYPE_RESULT_IDENTIFIER: &str = "f";
pub const GO_AST_INVENTORY_SCRATCH_FUNC_TYPE_RESULT_SIGNATURE_POLICY: &str = "no_params_no_results";
pub const GO_AST_INVENTORY_SCRATCH_FUNC_TYPE_RESULT_RETURN_IDENTIFIER: &str = "nil";
pub const GO_AST_INVENTORY_SCRATCH_FUNC_TYPE_RESULT_GO_VERSION: &str = "go1.24";
pub const GO_AST_INVENTORY_SCRATCH_FUNC_TYPE_RESULT_CONTRACT: &str = "token-only GoSourceFile function-type result scratch seed selected stepwise from raw state-machine admissible masks";
pub const GO_AST_INVENTORY_SCRATCH_ARRAY_TYPE_RETURN_RENDERED_SOURCE: &str =
    "package p\n\nfunc f() []int {\n\treturn nil\n}\n";
pub const GO_AST_INVENTORY_SCRATCH_ARRAY_TYPE_RETURN_ROOT: &str = "GoSourceFile";
pub const GO_AST_INVENTORY_SCRATCH_ARRAY_TYPE_RETURN_SEED_ID: &str =
    "go_ast_inventory_scratch_array_type_return_seed";
pub const GO_AST_INVENTORY_SCRATCH_ARRAY_TYPE_RETURN_GRAMMAR_CATEGORY: &str =
    "function_return_array_type";
pub const GO_AST_INVENTORY_SCRATCH_ARRAY_TYPE_RETURN_PACKAGE_NAME: &str = "p";
pub const GO_AST_INVENTORY_SCRATCH_ARRAY_TYPE_RETURN_IDENTIFIER: &str = "f";
pub const GO_AST_INVENTORY_SCRATCH_ARRAY_TYPE_RETURN_ELEMENT_TYPE: &str = "int";
pub const GO_AST_INVENTORY_SCRATCH_ARRAY_TYPE_RETURN_RETURN_IDENTIFIER: &str = "nil";
pub const GO_AST_INVENTORY_SCRATCH_ARRAY_TYPE_RETURN_GO_VERSION: &str = "go1.24";
pub const GO_AST_INVENTORY_SCRATCH_ARRAY_TYPE_RETURN_CONTRACT: &str = "token-only GoSourceFile array-type return scratch seed selected stepwise from raw state-machine admissible masks";
pub const GO_AST_INVENTORY_SCRATCH_CHAN_TYPE_RETURN_RENDERED_SOURCE: &str =
    "package p\n\nfunc f() chan int {\n\treturn nil\n}\n";
pub const GO_AST_INVENTORY_SCRATCH_CHAN_TYPE_RETURN_ROOT: &str = "GoSourceFile";
pub const GO_AST_INVENTORY_SCRATCH_CHAN_TYPE_RETURN_SEED_ID: &str =
    "go_ast_inventory_scratch_chan_type_return_seed";
pub const GO_AST_INVENTORY_SCRATCH_CHAN_TYPE_RETURN_GRAMMAR_CATEGORY: &str =
    "function_return_channel_type";
pub const GO_AST_INVENTORY_SCRATCH_CHAN_TYPE_RETURN_PACKAGE_NAME: &str = "p";
pub const GO_AST_INVENTORY_SCRATCH_CHAN_TYPE_RETURN_IDENTIFIER: &str = "f";
pub const GO_AST_INVENTORY_SCRATCH_CHAN_TYPE_RETURN_DIRECTION: &str = "SEND_RECV";
pub const GO_AST_INVENTORY_SCRATCH_CHAN_TYPE_RETURN_ELEMENT_TYPE: &str = "int";
pub const GO_AST_INVENTORY_SCRATCH_CHAN_TYPE_RETURN_RETURN_IDENTIFIER: &str = "nil";
pub const GO_AST_INVENTORY_SCRATCH_CHAN_TYPE_RETURN_GO_VERSION: &str = "go1.24";
pub const GO_AST_INVENTORY_SCRATCH_CHAN_TYPE_RETURN_CONTRACT: &str = "token-only GoSourceFile channel-type return scratch seed selected stepwise from raw state-machine admissible masks";
pub const GO_AST_INVENTORY_SCRATCH_SELECTOR_CALL_RENDERED_SOURCE: &str =
    "package p\n\nfunc f() int {\n\treturn m.N()\n}\n";
pub const GO_AST_INVENTORY_SCRATCH_SELECTOR_CALL_ROOT: &str = "GoSourceFile";
pub const GO_AST_INVENTORY_SCRATCH_SELECTOR_CALL_SEED_ID: &str =
    "go_ast_inventory_scratch_selector_call_seed";
pub const GO_AST_INVENTORY_SCRATCH_SELECTOR_CALL_GRAMMAR_CATEGORY: &str =
    "selector_call_member_access_expression";
pub const GO_AST_INVENTORY_SCRATCH_SELECTOR_CALL_PACKAGE_NAME: &str = "p";
pub const GO_AST_INVENTORY_SCRATCH_SELECTOR_CALL_IDENTIFIER: &str = "f";
pub const GO_AST_INVENTORY_SCRATCH_SELECTOR_CALL_RESULT_TYPE: &str = "int";
pub const GO_AST_INVENTORY_SCRATCH_SELECTOR_CALL_RECEIVER: &str = "m";
pub const GO_AST_INVENTORY_SCRATCH_SELECTOR_CALL_SELECTOR: &str = "N";
pub const GO_AST_INVENTORY_SCRATCH_SELECTOR_CALL_GO_VERSION: &str = "go1.24";
pub const GO_AST_INVENTORY_SCRATCH_SELECTOR_CALL_CONTRACT: &str = "token-only GoSourceFile selector-call/member-access scratch seed selected stepwise from raw state-machine admissible masks";
pub const GO_AST_INVENTORY_SCRATCH_INDEX_EXPR_RENDERED_SOURCE: &str =
    "package p\n\nfunc f() int {\n\treturn xs[0]\n}\n";
pub const GO_AST_INVENTORY_SCRATCH_INDEX_EXPR_ROOT: &str = "GoSourceFile";
pub const GO_AST_INVENTORY_SCRATCH_INDEX_EXPR_SEED_ID: &str =
    "go_ast_inventory_scratch_index_expr_seed";
pub const GO_AST_INVENTORY_SCRATCH_INDEX_EXPR_GRAMMAR_CATEGORY: &str = "index_expression";
pub const GO_AST_INVENTORY_SCRATCH_INDEX_EXPR_PACKAGE_NAME: &str = "p";
pub const GO_AST_INVENTORY_SCRATCH_INDEX_EXPR_IDENTIFIER: &str = "f";
pub const GO_AST_INVENTORY_SCRATCH_INDEX_EXPR_RESULT_TYPE: &str = "int";
pub const GO_AST_INVENTORY_SCRATCH_INDEX_EXPR_TARGET_IDENTIFIER: &str = "xs";
pub const GO_AST_INVENTORY_SCRATCH_INDEX_EXPR_INDEX_LITERAL_KIND: &str = "INT";
pub const GO_AST_INVENTORY_SCRATCH_INDEX_EXPR_INDEX_LITERAL_VALUE: &str = "0";
pub const GO_AST_INVENTORY_SCRATCH_INDEX_EXPR_GO_VERSION: &str = "go1.24";
pub const GO_AST_INVENTORY_SCRATCH_INDEX_EXPR_CONTRACT: &str = "token-only GoSourceFile index-expression scratch seed selected stepwise from raw state-machine admissible masks";
pub const GO_AST_INVENTORY_SCRATCH_INDEX_LIST_EXPR_RENDERED_SOURCE: &str =
    "package p\n\nfunc f() int {\n\treturn g[int, string]\n}\n";
pub const GO_AST_INVENTORY_SCRATCH_INDEX_LIST_EXPR_ROOT: &str = "GoSourceFile";
pub const GO_AST_INVENTORY_SCRATCH_INDEX_LIST_EXPR_SEED_ID: &str =
    "go_ast_inventory_scratch_index_list_expr_seed";
pub const GO_AST_INVENTORY_SCRATCH_INDEX_LIST_EXPR_GRAMMAR_CATEGORY: &str = "index_list_expression";
pub const GO_AST_INVENTORY_SCRATCH_INDEX_LIST_EXPR_PACKAGE_NAME: &str = "p";
pub const GO_AST_INVENTORY_SCRATCH_INDEX_LIST_EXPR_IDENTIFIER: &str = "f";
pub const GO_AST_INVENTORY_SCRATCH_INDEX_LIST_EXPR_RESULT_TYPE: &str = "int";
pub const GO_AST_INVENTORY_SCRATCH_INDEX_LIST_EXPR_TARGET_IDENTIFIER: &str = "g";
pub const GO_AST_INVENTORY_SCRATCH_INDEX_LIST_EXPR_FIRST_INDEX_IDENTIFIER: &str = "int";
pub const GO_AST_INVENTORY_SCRATCH_INDEX_LIST_EXPR_SECOND_INDEX_IDENTIFIER: &str = "string";
pub const GO_AST_INVENTORY_SCRATCH_INDEX_LIST_EXPR_GO_VERSION: &str = "go1.24";
pub const GO_AST_INVENTORY_SCRATCH_INDEX_LIST_EXPR_CONTRACT: &str = "token-only GoSourceFile index-list-expression scratch seed selected stepwise from raw state-machine admissible masks";
pub const GO_AST_INVENTORY_SCRATCH_SLICE_EXPR_RENDERED_SOURCE: &str =
    "package p\n\nfunc f() int {\n\treturn xs[1:2]\n}\n";
pub const GO_AST_INVENTORY_SCRATCH_SLICE_EXPR_ROOT: &str = "GoSourceFile";
pub const GO_AST_INVENTORY_SCRATCH_SLICE_EXPR_SEED_ID: &str =
    "go_ast_inventory_scratch_slice_expr_seed";
pub const GO_AST_INVENTORY_SCRATCH_SLICE_EXPR_GRAMMAR_CATEGORY: &str = "slice_expression";
pub const GO_AST_INVENTORY_SCRATCH_SLICE_EXPR_PACKAGE_NAME: &str = "p";
pub const GO_AST_INVENTORY_SCRATCH_SLICE_EXPR_IDENTIFIER: &str = "f";
pub const GO_AST_INVENTORY_SCRATCH_SLICE_EXPR_RESULT_TYPE: &str = "int";
pub const GO_AST_INVENTORY_SCRATCH_SLICE_EXPR_TARGET_IDENTIFIER: &str = "xs";
pub const GO_AST_INVENTORY_SCRATCH_SLICE_EXPR_LOW_LITERAL_KIND: &str = "INT";
pub const GO_AST_INVENTORY_SCRATCH_SLICE_EXPR_LOW_LITERAL_VALUE: &str = "1";
pub const GO_AST_INVENTORY_SCRATCH_SLICE_EXPR_HIGH_LITERAL_KIND: &str = "INT";
pub const GO_AST_INVENTORY_SCRATCH_SLICE_EXPR_HIGH_LITERAL_VALUE: &str = "2";
pub const GO_AST_INVENTORY_SCRATCH_SLICE_EXPR_THREE_INDEX: bool = false;
pub const GO_AST_INVENTORY_SCRATCH_SLICE_EXPR_GO_VERSION: &str = "go1.24";
pub const GO_AST_INVENTORY_SCRATCH_SLICE_EXPR_CONTRACT: &str = "token-only GoSourceFile slice-expression scratch seed selected stepwise from raw state-machine admissible masks";
pub const GO_AST_INVENTORY_SCRATCH_STAR_EXPR_RENDERED_SOURCE: &str =
    "package p\n\nfunc f() int {\n\treturn *x\n}\n";
pub const GO_AST_INVENTORY_SCRATCH_STAR_EXPR_ROOT: &str = "GoSourceFile";
pub const GO_AST_INVENTORY_SCRATCH_STAR_EXPR_SEED_ID: &str =
    "go_ast_inventory_scratch_star_expr_seed";
pub const GO_AST_INVENTORY_SCRATCH_STAR_EXPR_GRAMMAR_CATEGORY: &str = "star_expression";
pub const GO_AST_INVENTORY_SCRATCH_STAR_EXPR_PACKAGE_NAME: &str = "p";
pub const GO_AST_INVENTORY_SCRATCH_STAR_EXPR_IDENTIFIER: &str = "f";
pub const GO_AST_INVENTORY_SCRATCH_STAR_EXPR_RESULT_TYPE: &str = "int";
pub const GO_AST_INVENTORY_SCRATCH_STAR_EXPR_TARGET_IDENTIFIER: &str = "x";
pub const GO_AST_INVENTORY_SCRATCH_STAR_EXPR_GO_VERSION: &str = "go1.24";
pub const GO_AST_INVENTORY_SCRATCH_STAR_EXPR_CONTRACT: &str = "token-only GoSourceFile star-expression scratch seed selected stepwise from raw state-machine admissible masks";
pub const GO_AST_INVENTORY_SCRATCH_TYPE_ASSERT_EXPR_RENDERED_SOURCE: &str =
    "package p\n\nfunc f() int {\n\treturn x.(int)\n}\n";
pub const GO_AST_INVENTORY_SCRATCH_TYPE_ASSERT_EXPR_ROOT: &str = "GoSourceFile";
pub const GO_AST_INVENTORY_SCRATCH_TYPE_ASSERT_EXPR_SEED_ID: &str =
    "go_ast_inventory_scratch_type_assert_expr_seed";
pub const GO_AST_INVENTORY_SCRATCH_TYPE_ASSERT_EXPR_GRAMMAR_CATEGORY: &str =
    "type_assertion_expression";
pub const GO_AST_INVENTORY_SCRATCH_TYPE_ASSERT_EXPR_PACKAGE_NAME: &str = "p";
pub const GO_AST_INVENTORY_SCRATCH_TYPE_ASSERT_EXPR_IDENTIFIER: &str = "f";
pub const GO_AST_INVENTORY_SCRATCH_TYPE_ASSERT_EXPR_RESULT_TYPE: &str = "int";
pub const GO_AST_INVENTORY_SCRATCH_TYPE_ASSERT_EXPR_TARGET_IDENTIFIER: &str = "x";
pub const GO_AST_INVENTORY_SCRATCH_TYPE_ASSERT_EXPR_ASSERTED_TYPE: &str = "int";
pub const GO_AST_INVENTORY_SCRATCH_TYPE_ASSERT_EXPR_GO_VERSION: &str = "go1.24";
pub const GO_AST_INVENTORY_SCRATCH_TYPE_ASSERT_EXPR_CONTRACT: &str = "token-only GoSourceFile type-assertion expression scratch seed selected stepwise from raw state-machine admissible masks";
pub const GO_AST_INVENTORY_SCRATCH_TYPE_SWITCH_STMT_RENDERED_SOURCE: &str =
    "package p\n\nfunc f() {\n\tswitch x.(type) {\n\t}\n}\n";
pub const GO_AST_INVENTORY_SCRATCH_TYPE_SWITCH_STMT_ROOT: &str = "GoSourceFile";
pub const GO_AST_INVENTORY_SCRATCH_TYPE_SWITCH_STMT_SEED_ID: &str =
    "go_ast_inventory_scratch_type_switch_stmt_seed";
pub const GO_AST_INVENTORY_SCRATCH_TYPE_SWITCH_STMT_GRAMMAR_CATEGORY: &str =
    "type_switch_statement";
pub const GO_AST_INVENTORY_SCRATCH_TYPE_SWITCH_STMT_PACKAGE_NAME: &str = "p";
pub const GO_AST_INVENTORY_SCRATCH_TYPE_SWITCH_STMT_IDENTIFIER: &str = "f";
pub const GO_AST_INVENTORY_SCRATCH_TYPE_SWITCH_STMT_TARGET_IDENTIFIER: &str = "x";
pub const GO_AST_INVENTORY_SCRATCH_TYPE_SWITCH_STMT_GO_VERSION: &str = "go1.24";
pub const GO_AST_INVENTORY_SCRATCH_TYPE_SWITCH_STMT_CONTRACT: &str = "token-only GoSourceFile type-switch statement scratch seed selected stepwise from raw state-machine admissible masks";
pub const GO_AST_INVENTORY_SCRATCH_STRUCT_TYPE_RENDERED_SOURCE: &str =
    "package p\n\ntype S struct {\n\tX int\n}\n";
pub const GO_AST_INVENTORY_SCRATCH_STRUCT_TYPE_ROOT: &str = "GoSourceFile";
pub const GO_AST_INVENTORY_SCRATCH_STRUCT_TYPE_SEED_ID: &str =
    "go_ast_inventory_scratch_struct_type_seed";
pub const GO_AST_INVENTORY_SCRATCH_STRUCT_TYPE_GRAMMAR_CATEGORY: &str = "struct_type";
pub const GO_AST_INVENTORY_SCRATCH_STRUCT_TYPE_PACKAGE_NAME: &str = "p";
pub const GO_AST_INVENTORY_SCRATCH_STRUCT_TYPE_TYPE_NAME: &str = "S";
pub const GO_AST_INVENTORY_SCRATCH_STRUCT_TYPE_FIELD_NAME: &str = "X";
pub const GO_AST_INVENTORY_SCRATCH_STRUCT_TYPE_FIELD_TYPE: &str = "int";
pub const GO_AST_INVENTORY_SCRATCH_STRUCT_TYPE_GO_VERSION: &str = "go1.24";
pub const GO_AST_INVENTORY_SCRATCH_STRUCT_TYPE_CONTRACT: &str = "token-only GoSourceFile struct-type scratch seed selected stepwise from raw state-machine admissible masks";
pub const GO_AST_INVENTORY_SCRATCH_INTERFACE_TYPE_RENDERED_SOURCE: &str =
    "package p\n\nvar x interface{}\n";
pub const GO_AST_INVENTORY_SCRATCH_INTERFACE_TYPE_ROOT: &str = "GoSourceFile";
pub const GO_AST_INVENTORY_SCRATCH_INTERFACE_TYPE_SEED_ID: &str =
    "go_ast_inventory_scratch_interface_type_seed";
pub const GO_AST_INVENTORY_SCRATCH_INTERFACE_TYPE_GRAMMAR_CATEGORY: &str = "interface_type";
pub const GO_AST_INVENTORY_SCRATCH_INTERFACE_TYPE_PACKAGE_NAME: &str = "p";
pub const GO_AST_INVENTORY_SCRATCH_INTERFACE_TYPE_IDENTIFIER: &str = "x";
pub const GO_AST_INVENTORY_SCRATCH_INTERFACE_TYPE_GO_VERSION: &str = "go1.24";
pub const GO_AST_INVENTORY_SCRATCH_INTERFACE_TYPE_CONTRACT: &str = "token-only GoSourceFile interface-type scratch seed selected stepwise from raw state-machine admissible masks";
pub const GO_AST_INVENTORY_SCRATCH_KEY_VALUE_EXPR_RENDERED_SOURCE: &str =
    "package p\n\nfunc f() map[string]int {\n\treturn map[string]int{\"k\": 1}\n}\n";
pub const GO_AST_INVENTORY_SCRATCH_KEY_VALUE_EXPR_ROOT: &str = "GoSourceFile";
pub const GO_AST_INVENTORY_SCRATCH_KEY_VALUE_EXPR_SEED_ID: &str =
    "go_ast_inventory_scratch_key_value_expr_seed";
pub const GO_AST_INVENTORY_SCRATCH_KEY_VALUE_EXPR_GRAMMAR_CATEGORY: &str = "key_value_expression";
pub const GO_AST_INVENTORY_SCRATCH_KEY_VALUE_EXPR_PACKAGE_NAME: &str = "p";
pub const GO_AST_INVENTORY_SCRATCH_KEY_VALUE_EXPR_IDENTIFIER: &str = "f";
pub const GO_AST_INVENTORY_SCRATCH_KEY_VALUE_EXPR_KEY_TYPE: &str = "string";
pub const GO_AST_INVENTORY_SCRATCH_KEY_VALUE_EXPR_VALUE_TYPE: &str = "int";
pub const GO_AST_INVENTORY_SCRATCH_KEY_VALUE_EXPR_KEY_LITERAL_KIND: &str = "STRING";
pub const GO_AST_INVENTORY_SCRATCH_KEY_VALUE_EXPR_KEY_LITERAL_VALUE: &str = "\"k\"";
pub const GO_AST_INVENTORY_SCRATCH_KEY_VALUE_EXPR_VALUE_LITERAL_KIND: &str = "INT";
pub const GO_AST_INVENTORY_SCRATCH_KEY_VALUE_EXPR_VALUE_LITERAL_VALUE: &str = "1";
pub const GO_AST_INVENTORY_SCRATCH_KEY_VALUE_EXPR_GO_VERSION: &str = "go1.24";
pub const GO_AST_INVENTORY_SCRATCH_KEY_VALUE_EXPR_CONTRACT: &str = "token-only GoSourceFile key-value-expression scratch seed selected stepwise from raw state-machine admissible masks";
pub const GO_AST_INVENTORY_SCRATCH_ASSIGNMENT_RENDERED_SOURCE: &str =
    "package p\n\nfunc f() int {\n\tx = 1\n\treturn x\n}\n";
pub const GO_AST_INVENTORY_SCRATCH_ASSIGNMENT_ROOT: &str = "GoSourceFile";
pub const GO_AST_INVENTORY_SCRATCH_ASSIGNMENT_SEED_ID: &str =
    "go_ast_inventory_scratch_assignment_seed";
pub const GO_AST_INVENTORY_SCRATCH_ASSIGNMENT_GRAMMAR_CATEGORY: &str = "assignment_statement";
pub const GO_AST_INVENTORY_SCRATCH_ASSIGNMENT_PACKAGE_NAME: &str = "p";
pub const GO_AST_INVENTORY_SCRATCH_ASSIGNMENT_IDENTIFIER: &str = "f";
pub const GO_AST_INVENTORY_SCRATCH_ASSIGNMENT_RESULT_TYPE: &str = "int";
pub const GO_AST_INVENTORY_SCRATCH_ASSIGNMENT_LHS_IDENTIFIER: &str = "x";
pub const GO_AST_INVENTORY_SCRATCH_ASSIGNMENT_OPERATOR: &str = "=";
pub const GO_AST_INVENTORY_SCRATCH_ASSIGNMENT_LITERAL_KIND: &str = "INT";
pub const GO_AST_INVENTORY_SCRATCH_ASSIGNMENT_LITERAL_VALUE: &str = "1";
pub const GO_AST_INVENTORY_SCRATCH_ASSIGNMENT_RETURN_IDENTIFIER: &str = "x";
pub const GO_AST_INVENTORY_SCRATCH_ASSIGNMENT_GO_VERSION: &str = "go1.24";
pub const GO_AST_INVENTORY_SCRATCH_ASSIGNMENT_CONTRACT: &str = "token-only GoSourceFile assignment-statement scratch seed selected stepwise from raw state-machine admissible masks";
pub const GO_AST_INVENTORY_SCRATCH_BINARY_EXPR_RENDERED_SOURCE: &str =
    "package p\n\nfunc f() int {\n\treturn 2 + 3\n}\n";
pub const GO_AST_INVENTORY_SCRATCH_BINARY_EXPR_ROOT: &str = "GoSourceFile";
pub const GO_AST_INVENTORY_SCRATCH_BINARY_EXPR_SEED_ID: &str =
    "go_ast_inventory_scratch_binary_expr_seed";
pub const GO_AST_INVENTORY_SCRATCH_BINARY_EXPR_GRAMMAR_CATEGORY: &str = "binary_expression";
pub const GO_AST_INVENTORY_SCRATCH_BINARY_EXPR_PACKAGE_NAME: &str = "p";
pub const GO_AST_INVENTORY_SCRATCH_BINARY_EXPR_IDENTIFIER: &str = "f";
pub const GO_AST_INVENTORY_SCRATCH_BINARY_EXPR_RESULT_TYPE: &str = "int";
pub const GO_AST_INVENTORY_SCRATCH_BINARY_EXPR_LEFT_LITERAL_KIND: &str = "INT";
pub const GO_AST_INVENTORY_SCRATCH_BINARY_EXPR_LEFT_LITERAL_VALUE: &str = "2";
pub const GO_AST_INVENTORY_SCRATCH_BINARY_EXPR_OPERATOR: &str = "+";
pub const GO_AST_INVENTORY_SCRATCH_BINARY_EXPR_RIGHT_LITERAL_KIND: &str = "INT";
pub const GO_AST_INVENTORY_SCRATCH_BINARY_EXPR_RIGHT_LITERAL_VALUE: &str = "3";
pub const GO_AST_INVENTORY_SCRATCH_BINARY_EXPR_GO_VERSION: &str = "go1.24";
pub const GO_AST_INVENTORY_SCRATCH_BINARY_EXPR_CONTRACT: &str = "token-only GoSourceFile binary-expression scratch seed selected stepwise from raw state-machine admissible masks";
pub const GO_AST_INVENTORY_SCRATCH_UNARY_EXPR_RENDERED_SOURCE: &str =
    "package p\n\nfunc f() int {\n\treturn -x\n}\n";
pub const GO_AST_INVENTORY_SCRATCH_UNARY_EXPR_ROOT: &str = "GoSourceFile";
pub const GO_AST_INVENTORY_SCRATCH_UNARY_EXPR_SEED_ID: &str =
    "go_ast_inventory_scratch_unary_expr_seed";
pub const GO_AST_INVENTORY_SCRATCH_UNARY_EXPR_GRAMMAR_CATEGORY: &str = "unary_expression";
pub const GO_AST_INVENTORY_SCRATCH_UNARY_EXPR_PACKAGE_NAME: &str = "p";
pub const GO_AST_INVENTORY_SCRATCH_UNARY_EXPR_IDENTIFIER: &str = "f";
pub const GO_AST_INVENTORY_SCRATCH_UNARY_EXPR_RESULT_TYPE: &str = "int";
pub const GO_AST_INVENTORY_SCRATCH_UNARY_EXPR_OPERATOR: &str = "-";
pub const GO_AST_INVENTORY_SCRATCH_UNARY_EXPR_TARGET_IDENTIFIER: &str = "x";
pub const GO_AST_INVENTORY_SCRATCH_UNARY_EXPR_GO_VERSION: &str = "go1.24";
pub const GO_AST_INVENTORY_SCRATCH_UNARY_EXPR_CONTRACT: &str = "token-only GoSourceFile unary-expression scratch seed selected stepwise from raw state-machine admissible masks";
pub const GO_AST_INVENTORY_SCRATCH_PAREN_EXPR_RENDERED_SOURCE: &str =
    "package p\n\nfunc f() int {\n\treturn (1)\n}\n";
pub const GO_AST_INVENTORY_SCRATCH_PAREN_EXPR_ROOT: &str = "GoSourceFile";
pub const GO_AST_INVENTORY_SCRATCH_PAREN_EXPR_SEED_ID: &str =
    "go_ast_inventory_scratch_paren_expr_seed";
pub const GO_AST_INVENTORY_SCRATCH_PAREN_EXPR_GRAMMAR_CATEGORY: &str = "parenthesized_expression";
pub const GO_AST_INVENTORY_SCRATCH_PAREN_EXPR_PACKAGE_NAME: &str = "p";
pub const GO_AST_INVENTORY_SCRATCH_PAREN_EXPR_IDENTIFIER: &str = "f";
pub const GO_AST_INVENTORY_SCRATCH_PAREN_EXPR_RESULT_TYPE: &str = "int";
pub const GO_AST_INVENTORY_SCRATCH_PAREN_EXPR_LITERAL_KIND: &str = "INT";
pub const GO_AST_INVENTORY_SCRATCH_PAREN_EXPR_LITERAL_VALUE: &str = "1";
pub const GO_AST_INVENTORY_SCRATCH_PAREN_EXPR_GO_VERSION: &str = "go1.24";
pub const GO_AST_INVENTORY_SCRATCH_PAREN_EXPR_CONTRACT: &str = "token-only GoSourceFile parenthesized-expression scratch seed selected stepwise from raw state-machine admissible masks";
pub const GO_AST_INVENTORY_SCRATCH_COMPOSITE_LIT_RENDERED_SOURCE: &str =
    "package p\n\nfunc f() []int {\n\treturn []int{1}\n}\n";
pub const GO_AST_INVENTORY_SCRATCH_COMPOSITE_LIT_ROOT: &str = "GoSourceFile";
pub const GO_AST_INVENTORY_SCRATCH_COMPOSITE_LIT_SEED_ID: &str =
    "go_ast_inventory_scratch_composite_lit_seed";
pub const GO_AST_INVENTORY_SCRATCH_COMPOSITE_LIT_GRAMMAR_CATEGORY: &str = "composite_literal";
pub const GO_AST_INVENTORY_SCRATCH_COMPOSITE_LIT_PACKAGE_NAME: &str = "p";
pub const GO_AST_INVENTORY_SCRATCH_COMPOSITE_LIT_IDENTIFIER: &str = "f";
pub const GO_AST_INVENTORY_SCRATCH_COMPOSITE_LIT_ELEMENT_TYPE: &str = "int";
pub const GO_AST_INVENTORY_SCRATCH_COMPOSITE_LIT_LITERAL_KIND: &str = "INT";
pub const GO_AST_INVENTORY_SCRATCH_COMPOSITE_LIT_LITERAL_VALUE: &str = "1";
pub const GO_AST_INVENTORY_SCRATCH_COMPOSITE_LIT_GO_VERSION: &str = "go1.24";
pub const GO_AST_INVENTORY_SCRATCH_COMPOSITE_LIT_CONTRACT: &str = "token-only GoSourceFile composite-literal scratch seed selected stepwise from raw state-machine admissible masks";
pub const GO_AST_INVENTORY_SCRATCH_ELLIPSIS_ARRAY_LITERAL_RENDERED_SOURCE: &str =
    "package p\n\nvar xs = [...]int{1}\n";
pub const GO_AST_INVENTORY_SCRATCH_ELLIPSIS_ARRAY_LITERAL_ROOT: &str = "GoSourceFile";
pub const GO_AST_INVENTORY_SCRATCH_ELLIPSIS_ARRAY_LITERAL_SEED_ID: &str =
    "go_ast_inventory_scratch_ellipsis_array_literal_seed";
pub const GO_AST_INVENTORY_SCRATCH_ELLIPSIS_ARRAY_LITERAL_GRAMMAR_CATEGORY: &str =
    "array_literal_ellipsis_type";
pub const GO_AST_INVENTORY_SCRATCH_ELLIPSIS_ARRAY_LITERAL_PACKAGE_NAME: &str = "p";
pub const GO_AST_INVENTORY_SCRATCH_ELLIPSIS_ARRAY_LITERAL_IDENTIFIER: &str = "xs";
pub const GO_AST_INVENTORY_SCRATCH_ELLIPSIS_ARRAY_LITERAL_ELEMENT_TYPE: &str = "int";
pub const GO_AST_INVENTORY_SCRATCH_ELLIPSIS_ARRAY_LITERAL_LITERAL_KIND: &str = "INT";
pub const GO_AST_INVENTORY_SCRATCH_ELLIPSIS_ARRAY_LITERAL_LITERAL_VALUE: &str = "1";
pub const GO_AST_INVENTORY_SCRATCH_ELLIPSIS_ARRAY_LITERAL_GO_VERSION: &str = "go1.24";
pub const GO_AST_INVENTORY_SCRATCH_ELLIPSIS_ARRAY_LITERAL_CONTRACT: &str = "token-only GoSourceFile array-literal Ellipsis scratch seed selected stepwise from raw state-machine admissible masks";
pub const GO_AST_INVENTORY_SCRATCH_FUNC_LIT_RENDERED_SOURCE: &str =
    "package p\n\nvar f = func() {\n}\n";
pub const GO_AST_INVENTORY_SCRATCH_FUNC_LIT_ROOT: &str = "GoSourceFile";
pub const GO_AST_INVENTORY_SCRATCH_FUNC_LIT_SEED_ID: &str =
    "go_ast_inventory_scratch_func_lit_seed";
pub const GO_AST_INVENTORY_SCRATCH_FUNC_LIT_GRAMMAR_CATEGORY: &str = "function_literal_expression";
pub const GO_AST_INVENTORY_SCRATCH_FUNC_LIT_PACKAGE_NAME: &str = "p";
pub const GO_AST_INVENTORY_SCRATCH_FUNC_LIT_IDENTIFIER: &str = "f";
pub const GO_AST_INVENTORY_SCRATCH_FUNC_LIT_GO_VERSION: &str = "go1.24";
pub const GO_AST_INVENTORY_SCRATCH_FUNC_LIT_CONTRACT: &str = "token-only GoSourceFile function-literal expression scratch seed selected stepwise from raw state-machine admissible masks";
pub const GO_AST_INVENTORY_SCRATCH_DECL_STMT_RENDERED_SOURCE: &str =
    "package p\n\nfunc f() int {\n\tvar x = 1\n\treturn x\n}\n";
pub const GO_AST_INVENTORY_SCRATCH_DECL_STMT_ROOT: &str = "GoSourceFile";
pub const GO_AST_INVENTORY_SCRATCH_DECL_STMT_SEED_ID: &str =
    "go_ast_inventory_scratch_decl_stmt_seed";
pub const GO_AST_INVENTORY_SCRATCH_DECL_STMT_GRAMMAR_CATEGORY: &str = "declaration_statement";
pub const GO_AST_INVENTORY_SCRATCH_DECL_STMT_PACKAGE_NAME: &str = "p";
pub const GO_AST_INVENTORY_SCRATCH_DECL_STMT_IDENTIFIER: &str = "f";
pub const GO_AST_INVENTORY_SCRATCH_DECL_STMT_RESULT_TYPE: &str = "int";
pub const GO_AST_INVENTORY_SCRATCH_DECL_STMT_DECL_IDENTIFIER: &str = "x";
pub const GO_AST_INVENTORY_SCRATCH_DECL_STMT_LITERAL_KIND: &str = "INT";
pub const GO_AST_INVENTORY_SCRATCH_DECL_STMT_LITERAL_VALUE: &str = "1";
pub const GO_AST_INVENTORY_SCRATCH_DECL_STMT_RETURN_IDENTIFIER: &str = "x";
pub const GO_AST_INVENTORY_SCRATCH_DECL_STMT_GO_VERSION: &str = "go1.24";
pub const GO_AST_INVENTORY_SCRATCH_DECL_STMT_CONTRACT: &str = "token-only GoSourceFile declaration-statement scratch seed selected stepwise from raw state-machine admissible masks";
pub const GO_AST_INVENTORY_SCRATCH_DEFER_STMT_RENDERED_SOURCE: &str =
    "package p\n\nfunc f() int {\n\tdefer m.N()\n\treturn 1\n}\n";
pub const GO_AST_INVENTORY_SCRATCH_DEFER_STMT_ROOT: &str = "GoSourceFile";
pub const GO_AST_INVENTORY_SCRATCH_DEFER_STMT_SEED_ID: &str =
    "go_ast_inventory_scratch_defer_stmt_seed";
pub const GO_AST_INVENTORY_SCRATCH_DEFER_STMT_GRAMMAR_CATEGORY: &str = "defer_statement";
pub const GO_AST_INVENTORY_SCRATCH_DEFER_STMT_PACKAGE_NAME: &str = "p";
pub const GO_AST_INVENTORY_SCRATCH_DEFER_STMT_IDENTIFIER: &str = "f";
pub const GO_AST_INVENTORY_SCRATCH_DEFER_STMT_RESULT_TYPE: &str = "int";
pub const GO_AST_INVENTORY_SCRATCH_DEFER_STMT_CALL_RECEIVER: &str = "m";
pub const GO_AST_INVENTORY_SCRATCH_DEFER_STMT_CALL_SELECTOR: &str = "N";
pub const GO_AST_INVENTORY_SCRATCH_DEFER_STMT_RETURN_LITERAL_KIND: &str = "INT";
pub const GO_AST_INVENTORY_SCRATCH_DEFER_STMT_RETURN_LITERAL_VALUE: &str = "1";
pub const GO_AST_INVENTORY_SCRATCH_DEFER_STMT_GO_VERSION: &str = "go1.24";
pub const GO_AST_INVENTORY_SCRATCH_DEFER_STMT_CONTRACT: &str = "token-only GoSourceFile defer-statement scratch seed selected stepwise from raw state-machine admissible masks";
pub const GO_AST_INVENTORY_SCRATCH_GO_STMT_RENDERED_SOURCE: &str =
    "package p\n\nfunc f() {\n\tgo m.N()\n}\n";
pub const GO_AST_INVENTORY_SCRATCH_GO_STMT_ROOT: &str = "GoSourceFile";
pub const GO_AST_INVENTORY_SCRATCH_GO_STMT_SEED_ID: &str = "go_ast_inventory_scratch_go_stmt_seed";
pub const GO_AST_INVENTORY_SCRATCH_GO_STMT_GRAMMAR_CATEGORY: &str = "go_statement";
pub const GO_AST_INVENTORY_SCRATCH_GO_STMT_PACKAGE_NAME: &str = "p";
pub const GO_AST_INVENTORY_SCRATCH_GO_STMT_IDENTIFIER: &str = "f";
pub const GO_AST_INVENTORY_SCRATCH_GO_STMT_CALL_RECEIVER: &str = "m";
pub const GO_AST_INVENTORY_SCRATCH_GO_STMT_CALL_SELECTOR: &str = "N";
pub const GO_AST_INVENTORY_SCRATCH_GO_STMT_GO_VERSION: &str = "go1.24";
pub const GO_AST_INVENTORY_SCRATCH_GO_STMT_CONTRACT: &str = "token-only GoSourceFile go-statement scratch seed selected stepwise from raw state-machine admissible masks";
pub const GO_AST_INVENTORY_SCRATCH_IF_STMT_RENDERED_SOURCE: &str = "package p\n\nfunc f() int {\n\tif x := 1; x {\n\t\treturn 1\n\t} else {\n\t\treturn 0\n\t}\n}\n";
pub const GO_AST_INVENTORY_SCRATCH_IF_STMT_ROOT: &str = "GoSourceFile";
pub const GO_AST_INVENTORY_SCRATCH_IF_STMT_SEED_ID: &str = "go_ast_inventory_scratch_if_stmt_seed";
pub const GO_AST_INVENTORY_SCRATCH_IF_STMT_GRAMMAR_CATEGORY: &str = "if_statement";
pub const GO_AST_INVENTORY_SCRATCH_IF_STMT_PACKAGE_NAME: &str = "p";
pub const GO_AST_INVENTORY_SCRATCH_IF_STMT_IDENTIFIER: &str = "f";
pub const GO_AST_INVENTORY_SCRATCH_IF_STMT_RESULT_TYPE: &str = "int";
pub const GO_AST_INVENTORY_SCRATCH_IF_STMT_INIT_IDENTIFIER: &str = "x";
pub const GO_AST_INVENTORY_SCRATCH_IF_STMT_INIT_OPERATOR: &str = ":=";
pub const GO_AST_INVENTORY_SCRATCH_IF_STMT_INIT_LITERAL_KIND: &str = "INT";
pub const GO_AST_INVENTORY_SCRATCH_IF_STMT_INIT_LITERAL_VALUE: &str = "1";
pub const GO_AST_INVENTORY_SCRATCH_IF_STMT_COND_IDENTIFIER: &str = "x";
pub const GO_AST_INVENTORY_SCRATCH_IF_STMT_BODY_RETURN_LITERAL_KIND: &str = "INT";
pub const GO_AST_INVENTORY_SCRATCH_IF_STMT_BODY_RETURN_LITERAL_VALUE: &str = "1";
pub const GO_AST_INVENTORY_SCRATCH_IF_STMT_ELSE_RETURN_LITERAL_KIND: &str = "INT";
pub const GO_AST_INVENTORY_SCRATCH_IF_STMT_ELSE_RETURN_LITERAL_VALUE: &str = "0";
pub const GO_AST_INVENTORY_SCRATCH_IF_STMT_GO_VERSION: &str = "go1.24";
pub const GO_AST_INVENTORY_SCRATCH_IF_STMT_CONTRACT: &str = "token-only GoSourceFile if-statement scratch seed selected stepwise from raw state-machine admissible masks";
pub const GO_AST_INVENTORY_SCRATCH_IMPORT_SPEC_RENDERED_SOURCE: &str =
    "package p\n\nimport x \"m\"\n";
pub const GO_AST_INVENTORY_SCRATCH_IMPORT_SPEC_ROOT: &str = "GoSourceFile";
pub const GO_AST_INVENTORY_SCRATCH_IMPORT_SPEC_SEED_ID: &str =
    "go_ast_inventory_scratch_import_spec_seed";
pub const GO_AST_INVENTORY_SCRATCH_IMPORT_SPEC_GRAMMAR_CATEGORY: &str = "import_spec_alias";
pub const GO_AST_INVENTORY_SCRATCH_IMPORT_SPEC_PACKAGE_NAME: &str = "p";
pub const GO_AST_INVENTORY_SCRATCH_IMPORT_SPEC_ALIAS: &str = "x";
pub const GO_AST_INVENTORY_SCRATCH_IMPORT_SPEC_PATH_LITERAL_KIND: &str = "STRING";
pub const GO_AST_INVENTORY_SCRATCH_IMPORT_SPEC_PATH_LITERAL_VALUE: &str = "\"m\"";
pub const GO_AST_INVENTORY_SCRATCH_IMPORT_SPEC_GO_VERSION: &str = "go1.24";
pub const GO_AST_INVENTORY_SCRATCH_IMPORT_SPEC_CONTRACT: &str = "token-only GoSourceFile import-spec alias scratch seed selected stepwise from raw state-machine admissible masks";
pub const GO_AST_INVENTORY_SCRATCH_EMPTY_STMT_RENDERED_SOURCE: &str =
    "package p\n\nfunc f() {\n\n}\n";
pub const GO_AST_INVENTORY_SCRATCH_EMPTY_STMT_ROOT: &str = "GoSourceFile";
pub const GO_AST_INVENTORY_SCRATCH_EMPTY_STMT_SEED_ID: &str =
    "go_ast_inventory_scratch_empty_stmt_seed";
pub const GO_AST_INVENTORY_SCRATCH_EMPTY_STMT_GRAMMAR_CATEGORY: &str = "empty_statement";
pub const GO_AST_INVENTORY_SCRATCH_EMPTY_STMT_PACKAGE_NAME: &str = "p";
pub const GO_AST_INVENTORY_SCRATCH_EMPTY_STMT_IDENTIFIER: &str = "f";
pub const GO_AST_INVENTORY_SCRATCH_EMPTY_STMT_IMPLICIT: bool = false;
pub const GO_AST_INVENTORY_SCRATCH_EMPTY_STMT_GO_VERSION: &str = "go1.24";
pub const GO_AST_INVENTORY_SCRATCH_EMPTY_STMT_CONTRACT: &str = "token-only GoSourceFile explicit empty-statement scratch seed selected stepwise from raw state-machine admissible masks";
pub const GO_AST_INVENTORY_SCRATCH_EXPR_STMT_RENDERED_SOURCE: &str =
    "package p\n\nfunc f() {\n\tm.N()\n}\n";
pub const GO_AST_INVENTORY_SCRATCH_EXPR_STMT_ROOT: &str = "GoSourceFile";
pub const GO_AST_INVENTORY_SCRATCH_EXPR_STMT_SEED_ID: &str =
    "go_ast_inventory_scratch_expr_stmt_seed";
pub const GO_AST_INVENTORY_SCRATCH_EXPR_STMT_GRAMMAR_CATEGORY: &str = "expression_statement";
pub const GO_AST_INVENTORY_SCRATCH_EXPR_STMT_PACKAGE_NAME: &str = "p";
pub const GO_AST_INVENTORY_SCRATCH_EXPR_STMT_IDENTIFIER: &str = "f";
pub const GO_AST_INVENTORY_SCRATCH_EXPR_STMT_CALL_RECEIVER: &str = "m";
pub const GO_AST_INVENTORY_SCRATCH_EXPR_STMT_CALL_SELECTOR: &str = "N";
pub const GO_AST_INVENTORY_SCRATCH_EXPR_STMT_GO_VERSION: &str = "go1.24";
pub const GO_AST_INVENTORY_SCRATCH_EXPR_STMT_CONTRACT: &str = "token-only GoSourceFile expression-statement scratch seed selected stepwise from raw state-machine admissible masks";
pub const GO_AST_INVENTORY_SCRATCH_INC_DEC_STMT_RENDERED_SOURCE: &str =
    "package p\n\nfunc f() {\n\ti++\n}\n";
pub const GO_AST_INVENTORY_SCRATCH_INC_DEC_STMT_ROOT: &str = "GoSourceFile";
pub const GO_AST_INVENTORY_SCRATCH_INC_DEC_STMT_SEED_ID: &str =
    "go_ast_inventory_scratch_inc_dec_stmt_seed";
pub const GO_AST_INVENTORY_SCRATCH_INC_DEC_STMT_GRAMMAR_CATEGORY: &str =
    "increment_decrement_statement";
pub const GO_AST_INVENTORY_SCRATCH_INC_DEC_STMT_PACKAGE_NAME: &str = "p";
pub const GO_AST_INVENTORY_SCRATCH_INC_DEC_STMT_IDENTIFIER: &str = "f";
pub const GO_AST_INVENTORY_SCRATCH_INC_DEC_STMT_TARGET_IDENTIFIER: &str = "i";
pub const GO_AST_INVENTORY_SCRATCH_INC_DEC_STMT_TOKEN: &str = "++";
pub const GO_AST_INVENTORY_SCRATCH_INC_DEC_STMT_GO_VERSION: &str = "go1.24";
pub const GO_AST_INVENTORY_SCRATCH_INC_DEC_STMT_CONTRACT: &str = "token-only GoSourceFile increment/decrement statement scratch seed selected stepwise from raw state-machine admissible masks";
pub const GO_AST_INVENTORY_SCRATCH_BRANCH_STMT_RENDERED_SOURCE: &str =
    "package p\n\nfunc f() int {\n\tfor {\n\t\tbreak\n\t}\n\treturn 1\n}\n";
pub const GO_AST_INVENTORY_SCRATCH_BRANCH_STMT_ROOT: &str = "GoSourceFile";
pub const GO_AST_INVENTORY_SCRATCH_BRANCH_STMT_SEED_ID: &str =
    "go_ast_inventory_scratch_branch_stmt_seed";
pub const GO_AST_INVENTORY_SCRATCH_BRANCH_STMT_GRAMMAR_CATEGORY: &str = "branch_statement";
pub const GO_AST_INVENTORY_SCRATCH_BRANCH_STMT_PACKAGE_NAME: &str = "p";
pub const GO_AST_INVENTORY_SCRATCH_BRANCH_STMT_IDENTIFIER: &str = "f";
pub const GO_AST_INVENTORY_SCRATCH_BRANCH_STMT_RESULT_TYPE: &str = "int";
pub const GO_AST_INVENTORY_SCRATCH_BRANCH_STMT_TOKEN: &str = "break";
pub const GO_AST_INVENTORY_SCRATCH_BRANCH_STMT_RETURN_LITERAL_KIND: &str = "INT";
pub const GO_AST_INVENTORY_SCRATCH_BRANCH_STMT_RETURN_LITERAL_VALUE: &str = "1";
pub const GO_AST_INVENTORY_SCRATCH_BRANCH_STMT_GO_VERSION: &str = "go1.24";
pub const GO_AST_INVENTORY_SCRATCH_BRANCH_STMT_CONTRACT: &str = "token-only GoSourceFile branch-statement scratch seed selected stepwise from raw state-machine admissible masks";
pub const GO_AST_INVENTORY_SCRATCH_RANGE_STMT_RENDERED_SOURCE: &str =
    "package p\n\nfunc f() {\n\tfor i := range xs {\n\t}\n}\n";
pub const GO_AST_INVENTORY_SCRATCH_RANGE_STMT_ROOT: &str = "GoSourceFile";
pub const GO_AST_INVENTORY_SCRATCH_RANGE_STMT_SEED_ID: &str =
    "go_ast_inventory_scratch_range_stmt_seed";
pub const GO_AST_INVENTORY_SCRATCH_RANGE_STMT_GRAMMAR_CATEGORY: &str = "range_statement";
pub const GO_AST_INVENTORY_SCRATCH_RANGE_STMT_PACKAGE_NAME: &str = "p";
pub const GO_AST_INVENTORY_SCRATCH_RANGE_STMT_IDENTIFIER: &str = "f";
pub const GO_AST_INVENTORY_SCRATCH_RANGE_STMT_KEY_IDENTIFIER: &str = "i";
pub const GO_AST_INVENTORY_SCRATCH_RANGE_STMT_TOKEN: &str = ":=";
pub const GO_AST_INVENTORY_SCRATCH_RANGE_STMT_SOURCE_IDENTIFIER: &str = "xs";
pub const GO_AST_INVENTORY_SCRATCH_RANGE_STMT_GO_VERSION: &str = "go1.24";
pub const GO_AST_INVENTORY_SCRATCH_RANGE_STMT_CONTRACT: &str = "token-only GoSourceFile range-statement scratch seed selected stepwise from raw state-machine admissible masks";
pub const GO_AST_INVENTORY_SCRATCH_SEND_STMT_RENDERED_SOURCE: &str =
    "package p\n\nfunc f() {\n\tch <- x\n}\n";
pub const GO_AST_INVENTORY_SCRATCH_SEND_STMT_ROOT: &str = "GoSourceFile";
pub const GO_AST_INVENTORY_SCRATCH_SEND_STMT_SEED_ID: &str =
    "go_ast_inventory_scratch_send_stmt_seed";
pub const GO_AST_INVENTORY_SCRATCH_SEND_STMT_GRAMMAR_CATEGORY: &str = "send_statement";
pub const GO_AST_INVENTORY_SCRATCH_SEND_STMT_PACKAGE_NAME: &str = "p";
pub const GO_AST_INVENTORY_SCRATCH_SEND_STMT_IDENTIFIER: &str = "f";
pub const GO_AST_INVENTORY_SCRATCH_SEND_STMT_CHANNEL_IDENTIFIER: &str = "ch";
pub const GO_AST_INVENTORY_SCRATCH_SEND_STMT_VALUE_IDENTIFIER: &str = "x";
pub const GO_AST_INVENTORY_SCRATCH_SEND_STMT_GO_VERSION: &str = "go1.24";
pub const GO_AST_INVENTORY_SCRATCH_SEND_STMT_CONTRACT: &str = "token-only GoSourceFile send-statement scratch seed selected stepwise from raw state-machine admissible masks";
pub const GO_AST_INVENTORY_SCRATCH_LABELED_STMT_RENDERED_SOURCE: &str =
    "package p\n\nfunc f() int {\nlabel:\n\treturn 1\n}\n";
pub const GO_AST_INVENTORY_SCRATCH_LABELED_STMT_ROOT: &str = "GoSourceFile";
pub const GO_AST_INVENTORY_SCRATCH_LABELED_STMT_SEED_ID: &str =
    "go_ast_inventory_scratch_labeled_stmt_seed";
pub const GO_AST_INVENTORY_SCRATCH_LABELED_STMT_GRAMMAR_CATEGORY: &str = "labeled_statement";
pub const GO_AST_INVENTORY_SCRATCH_LABELED_STMT_PACKAGE_NAME: &str = "p";
pub const GO_AST_INVENTORY_SCRATCH_LABELED_STMT_IDENTIFIER: &str = "f";
pub const GO_AST_INVENTORY_SCRATCH_LABELED_STMT_RESULT_TYPE: &str = "int";
pub const GO_AST_INVENTORY_SCRATCH_LABELED_STMT_LABEL_IDENTIFIER: &str = "label";
pub const GO_AST_INVENTORY_SCRATCH_LABELED_STMT_RETURN_LITERAL_KIND: &str = "INT";
pub const GO_AST_INVENTORY_SCRATCH_LABELED_STMT_RETURN_LITERAL_VALUE: &str = "1";
pub const GO_AST_INVENTORY_SCRATCH_LABELED_STMT_GO_VERSION: &str = "go1.24";
pub const GO_AST_INVENTORY_SCRATCH_LABELED_STMT_CONTRACT: &str = "token-only GoSourceFile labeled-statement scratch seed selected stepwise from raw state-machine admissible masks";
pub const GO_AST_INVENTORY_SCRATCH_CASE_CLAUSE_RENDERED_SOURCE: &str =
    "package p\n\nfunc f() int {\n\tswitch 1 {\n\tcase 1:\n\t\treturn 1\n\t}\n\treturn 1\n}\n";
pub const GO_AST_INVENTORY_SCRATCH_CASE_CLAUSE_ROOT: &str = "GoSourceFile";
pub const GO_AST_INVENTORY_SCRATCH_CASE_CLAUSE_SEED_ID: &str =
    "go_ast_inventory_scratch_case_clause_seed";
pub const GO_AST_INVENTORY_SCRATCH_CASE_CLAUSE_GRAMMAR_CATEGORY: &str = "case_clause";
pub const GO_AST_INVENTORY_SCRATCH_CASE_CLAUSE_PACKAGE_NAME: &str = "p";
pub const GO_AST_INVENTORY_SCRATCH_CASE_CLAUSE_IDENTIFIER: &str = "f";
pub const GO_AST_INVENTORY_SCRATCH_CASE_CLAUSE_RESULT_TYPE: &str = "int";
pub const GO_AST_INVENTORY_SCRATCH_CASE_CLAUSE_SWITCH_LITERAL_KIND: &str = "INT";
pub const GO_AST_INVENTORY_SCRATCH_CASE_CLAUSE_SWITCH_LITERAL_VALUE: &str = "1";
pub const GO_AST_INVENTORY_SCRATCH_CASE_CLAUSE_CASE_LITERAL_KIND: &str = "INT";
pub const GO_AST_INVENTORY_SCRATCH_CASE_CLAUSE_CASE_LITERAL_VALUE: &str = "1";
pub const GO_AST_INVENTORY_SCRATCH_CASE_CLAUSE_RETURN_LITERAL_KIND: &str = "INT";
pub const GO_AST_INVENTORY_SCRATCH_CASE_CLAUSE_RETURN_LITERAL_VALUE: &str = "1";
pub const GO_AST_INVENTORY_SCRATCH_CASE_CLAUSE_GO_VERSION: &str = "go1.24";
pub const GO_AST_INVENTORY_SCRATCH_CASE_CLAUSE_CONTRACT: &str = "token-only GoSourceFile case-clause scratch seed selected stepwise from raw state-machine admissible masks";
pub const GO_AST_INVENTORY_SCRATCH_COMM_CLAUSE_RENDERED_SOURCE: &str =
    "// tct: comm clause\n\npackage p\n\nfunc f() int {\n\tselect {\n\tdefault:\n\t}\n}\n";
pub const GO_AST_INVENTORY_SCRATCH_COMM_CLAUSE_ROOT: &str = "GoSourceFile";
pub const GO_AST_INVENTORY_SCRATCH_COMM_CLAUSE_SEED_ID: &str =
    "go_ast_inventory_scratch_comm_clause_seed";
pub const GO_AST_INVENTORY_SCRATCH_COMM_CLAUSE_GRAMMAR_CATEGORY: &str = "comm_clause";
pub const GO_AST_INVENTORY_SCRATCH_COMM_CLAUSE_PACKAGE_NAME: &str = "p";
pub const GO_AST_INVENTORY_SCRATCH_COMM_CLAUSE_IDENTIFIER: &str = "f";
pub const GO_AST_INVENTORY_SCRATCH_COMM_CLAUSE_RESULT_TYPE: &str = "int";
pub const GO_AST_INVENTORY_SCRATCH_COMM_CLAUSE_GO_VERSION: &str = "go1.24";
pub const GO_AST_INVENTORY_SCRATCH_COMM_CLAUSE_CONTRACT: &str = "token-only GoSourceFile select/default comm-clause scratch seed selected stepwise from raw state-machine admissible masks";
