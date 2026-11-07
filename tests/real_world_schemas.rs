// Real-world schema compatibility testing
// Tests TCT's ability to handle actual JSON schemas from popular open-source projects
// These schemas represent production patterns and complexity levels found in the wild

use serde_json::json;
use tct::TctCompiler;

/// Test simplified package.json schema patterns
/// Based on npm package.json schema from SchemaStore
#[test]
fn test_real_world_schemas_extended_npm_package_json_schema() {
    let package_schema = json!({
        "title": "PackageJson",
        "type": "object",
        "properties": {
            "name": {
                "type": "string",
                "pattern": "^(?:@[a-z0-9-*~][a-z0-9-*._~]*/)?[a-z0-9-~][a-z0-9-._~]*$"
            },
            "version": {
                "type": "string"
            },
            "description": {
                "type": "string"
            },
            "main": {
                "type": "string"
            },
            "scripts": {
                "type": "object",
                "patternProperties": {
                    "^[a-zA-Z0-9:_-]+$": {
                        "type": "string"
                    }
                }
            },
            "dependencies": {
                "type": "object",
                "patternProperties": {
                    ".*": {
                        "type": "string"
                    }
                }
            },
            "devDependencies": {
                "type": "object",
                "patternProperties": {
                    ".*": {
                        "type": "string"
                    }
                }
            },
            "repository": {
                "oneOf": [
                    {
                        "type": "string"
                    },
                    {
                        "type": "object",
                        "properties": {
                            "type": {"type": "string"},
                            "url": {"type": "string"},
                            "directory": {"type": "string"}
                        }
                    }
                ]
            },
            "license": {
                "oneOf": [
                    {
                        "type": "string"
                    },
                    {
                        "type": "object",
                        "properties": {
                            "type": {"type": "string"},
                            "url": {"type": "string"}
                        }
                    }
                ]
            }
        },
        "required": ["name", "version"]
    });

    let result = TctCompiler::new(package_schema.clone(), "PackageJson")
        .unwrap()
        .compile();
    assert!(result.is_ok(), "Package.json schema generation failed");

    let grammar = result.unwrap();
    let code = grammar.generate_code();
    assert!(code.is_ok(), "Package.json code generation failed");

    let generated_code = code.unwrap();
    assert!(!generated_code.is_empty());

    println!("✅ NPM package.json schema test passed");
}

/// Test simplified ESLint configuration schema patterns  
/// Based on ESLint configuration schema from SchemaStore
#[test]
fn test_real_world_schemas_extended_eslint_config_schema() {
    let eslint_schema = json!({
        "title": "ESLintConfig",
        "type": "object",
        "properties": {
            "env": {
                "type": "object",
                "properties": {
                    "browser": {"type": "boolean"},
                    "node": {"type": "boolean"},
                    "es6": {"type": "boolean"},
                    "jest": {"type": "boolean"}
                }
            },
            "extends": {
                "oneOf": [
                    {"type": "string"},
                    {
                        "type": "array",
                        "items": {"type": "string"}
                    }
                ]
            },
            "parserOptions": {
                "type": "object",
                "properties": {
                    "ecmaVersion": {
                        "oneOf": [
                            {"type": "integer"},
                            {"type": "string"}
                        ]
                    },
                    "sourceType": {
                        "enum": ["script", "module"]
                    },
                    "ecmaFeatures": {
                        "type": "object",
                        "properties": {
                            "jsx": {"type": "boolean"},
                            "globalReturn": {"type": "boolean"},
                            "impliedStrict": {"type": "boolean"}
                        }
                    }
                }
            },
            "rules": {
                "type": "object",
                "patternProperties": {
                    "^[a-z-]+(/[a-z-]+)*$": {
                        "oneOf": [
                            {"enum": ["off", "warn", "error", "zero", "one", "two"]},
                            {
                                "type": "array",
                                "items": [
                                    {"enum": ["off", "warn", "error", "zero", "one", "two"]}
                                ],
                                "additionalItems": true
                            }
                        ]
                    }
                }
            },
            "plugins": {
                "type": "array",
                "items": {"type": "string"}
            },
            "globals": {
                "type": "object",
                "patternProperties": {
                    "^[a-zA-Z_$][a-zA-Z0-9_$]*$": {
                        "oneOf": [
                            {"type": "boolean"},
                            {"enum": ["readonly", "writable", "off"]}
                        ]
                    }
                }
            }
        }
    });

    let result = TctCompiler::new(eslint_schema.clone(), "ESLintConfig")
        .unwrap()
        .compile();
    if let Err(e) = &result {
        panic!("ESLint config schema generation failed: {}", e);
    }
    assert!(result.is_ok(), "ESLint config schema generation failed");

    let grammar = result.unwrap();
    let code = grammar.generate_code();
    assert!(code.is_ok(), "ESLint config code generation failed");

    let generated_code = code.unwrap();
    assert!(!generated_code.is_empty());

    println!("✅ ESLint configuration schema test passed");
}

/// Test simplified TypeScript configuration schema patterns
/// Based on tsconfig.json schema from SchemaStore  
#[test]
fn test_real_world_schemas_extended_typescript_config_schema() {
    let tsconfig_schema = json!({
        "title": "TSConfig",
        "type": "object",
        "properties": {
            "compilerOptions": {
                "type": "object",
                "properties": {
                    "target": {
                        "enum": ["es3", "es5", "es6", "es2015", "es2016", "es2017", "es2018", "es2019", "es2020", "esnext"]
                    },
                    "module": {
                        "enum": ["none", "commonjs", "amd", "system", "umd", "es6", "es2015", "es2020", "esnext"]
                    },
                    "lib": {
                        "type": "array",
                        "items": {
                            "enum": ["es5", "es6", "es2015", "es2016", "es2017", "es2018", "es2019", "es2020", "esnext", "dom", "webworker", "scripthost"]
                        }
                    },
                    "outDir": {"type": "string"},
                    "rootDir": {"type": "string"},
                    "strict": {"type": "boolean"},
                    "esModuleInterop": {"type": "boolean"},
                    "skipLibCheck": {"type": "boolean"},
                    "forceConsistentCasingInFileNames": {"type": "boolean"},
                    "declaration": {"type": "boolean"},
                    "sourceMap": {"type": "boolean"},
                    "moduleResolution": {
                        "enum": ["classic", "node"]
                    }
                }
            },
            "include": {
                "type": "array",
                "items": {"type": "string"}
            },
            "exclude": {
                "type": "array",
                "items": {"type": "string"}
            },
            "files": {
                "type": "array",
                "items": {"type": "string"}
            },
            "extends": {
                "oneOf": [
                    {"type": "string"},
                    {
                        "type": "array",
                        "items": {"type": "string"}
                    }
                ]
            }
        }
    });

    let result = TctCompiler::new(tsconfig_schema.clone(), "TSConfig")
        .unwrap()
        .compile();
    if let Err(e) = &result {
        panic!("TSConfig schema generation failed: {}", e);
    }
    assert!(result.is_ok(), "TSConfig schema generation failed");

    let grammar = result.unwrap();
    let code = grammar.generate_code();
    assert!(code.is_ok(), "TSConfig code generation failed");

    let generated_code = code.unwrap();
    assert!(!generated_code.is_empty());

    println!("✅ TypeScript configuration schema test passed");
}

/// Test simplified GitHub Actions workflow schema patterns
/// Based on GitHub workflow schema patterns
#[test]
fn test_real_world_schemas_extended_github_workflow_schema() {
    let workflow_schema = json!({
        "title": "GitHubWorkflow",
        "type": "object",
        "properties": {
            "name": {"type": "string"},
            "on": {
                "oneOf": [
                    {"type": "string"},
                    {
                        "type": "array",
                        "items": {"type": "string"}
                    },
                    {
                        "type": "object",
                        "properties": {
                            "push": {
                                "type": "object",
                                "properties": {
                                    "branches": {
                                        "type": "array",
                                        "items": {"type": "string"}
                                    },
                                    "paths": {
                                        "type": "array",
                                        "items": {"type": "string"}
                                    }
                                }
                            },
                            "pull_request": {
                                "type": "object",
                                "properties": {
                                    "branches": {
                                        "type": "array",
                                        "items": {"type": "string"}
                                    },
                                    "types": {
                                        "type": "array",
                                        "items": {
                                            "enum": ["opened", "closed", "synchronize", "reopened"]
                                        }
                                    }
                                }
                            }
                        }
                    }
                ]
            },
            "jobs": {
                "type": "object",
                "patternProperties": {
                    "^[a-zA-Z_][a-zA-Z0-9_-]*$": {
                        "type": "object",
                        "properties": {
                            "runs-on": {
                                "oneOf": [
                                    {"type": "string"},
                                    {
                                        "type": "array",
                                        "items": {"type": "string"}
                                    }
                                ]
                            },
                            "strategy": {
                                "type": "object",
                                "properties": {
                                    "matrix": {
                                        "type": "object",
                                        "patternProperties": {
                                            "^[a-zA-Z_][a-zA-Z0-9_-]*$": {
                                                "type": "array"
                                            }
                                        }
                                    }
                                }
                            },
                            "steps": {
                                "type": "array",
                                "items": {
                                    "type": "object",
                                    "properties": {
                                        "name": {"type": "string"},
                                        "uses": {"type": "string"},
                                        "run": {"type": "string"},
                                        "with": {
                                            "type": "object",
                                            "patternProperties": {
                                                ".*": {"type": "string"}
                                            }
                                        },
                                        "env": {
                                            "type": "object",
                                            "patternProperties": {
                                                "^[a-zA-Z_][a-zA-Z0-9_]*$": {"type": "string"}
                                            }
                                        }
                                    }
                                }
                            }
                        },
                        "required": ["runs-on", "steps"]
                    }
                }
            }
        },
        "required": ["jobs"]
    });

    let result = TctCompiler::new(workflow_schema.clone(), "GitHubWorkflow")
        .unwrap()
        .compile();
    assert!(result.is_ok(), "GitHub workflow schema generation failed");

    let grammar = result.unwrap();
    let code = grammar.generate_code();
    assert!(code.is_ok(), "GitHub workflow code generation failed");

    let generated_code = code.unwrap();
    assert!(!generated_code.is_empty());

    println!("✅ GitHub Actions workflow schema test passed");
}

/// Test API response schema patterns
/// Based on common REST API patterns
#[test]
fn test_real_world_schemas_extended_api_response_schema() {
    let api_schema = json!({
        "title": "APIResponse",
        "type": "object",
        "properties": {
            "success": {"type": "boolean"},
            "status": {
                "type": "integer",
                "minimum": 100,
                "maximum": 599
            },
            "message": {"type": "string"},
            "data": {
                "oneOf": [
                    {"type": "object"},
                    {"type": "array"}
                ]
            },
            "errors": {
                "type": "array",
                "items": {
                    "type": "object",
                    "properties": {
                        "code": {"type": "string"},
                        "field": {"type": "string"},
                        "message": {"type": "string"}
                    },
                    "required": ["code", "message"]
                }
            },
            "pagination": {
                "type": "object",
                "properties": {
                    "page": {"type": "integer", "minimum": 1},
                    "limit": {"type": "integer", "minimum": 1, "maximum": 1000},
                    "total": {"type": "integer", "minimum": 0},
                    "has_more": {"type": "boolean"}
                },
                "required": ["page", "limit", "total", "has_more"]
            },
            "metadata": {
                "type": "object",
                "properties": {
                    "timestamp": {"type": "string", "format": "date-time"},
                    "version": {"type": "string"},
                    "request_id": {"type": "string"}
                }
            }
        },
        "required": ["success", "status"]
    });

    let result = TctCompiler::new(api_schema.clone(), "APIResponse")
        .unwrap()
        .compile();
    if let Err(e) = &result {
        panic!("API response schema generation failed: {}", e);
    }
    assert!(result.is_ok(), "API response schema generation failed");

    let grammar = result.unwrap();
    let code = grammar.generate_code();
    assert!(code.is_ok(), "API response code generation failed");

    let generated_code = code.unwrap();
    assert!(!generated_code.is_empty());

    println!("✅ API response schema test passed");
}

/// Performance test with a complex real-world-style schema
#[test]
fn test_real_world_schemas_extended_complex_real_world_performance() {
    // Simulate a complex microservice configuration schema
    let complex_schema = json!({
        "title": "MicroserviceConfig",
        "type": "object",
        "properties": {
            "service": {
                "type": "object",
                "properties": {
                    "name": {"type": "string", "pattern": "^[a-z][a-z0-9-]*[a-z0-9]$"},
                    "version": {"type": "string"},
                    "environment": {"enum": ["development", "staging", "production"]},
                    "port": {"type": "integer", "minimum": 1024, "maximum": 65535},
                    "health_check": {
                        "type": "object",
                        "properties": {
                            "path": {"type": "string"},
                            "interval": {"type": "integer", "minimum": 1},
                            "timeout": {"type": "integer", "minimum": 1},
                            "retries": {"type": "integer", "minimum": 0, "maximum": 10}
                        },
                        "required": ["path", "interval", "timeout"]
                    }
                },
                "required": ["name", "version", "environment", "port"]
            },
            "database": {
                "type": "object",
                "properties": {
                    "type": {"enum": ["postgresql", "mysql", "mongodb", "redis"]},
                    "host": {"type": "string"},
                    "port": {"type": "integer"},
                    "name": {"type": "string"},
                    "ssl": {"type": "boolean"},
                    "pool": {
                        "type": "object",
                        "properties": {
                            "min": {"type": "integer", "minimum": 0},
                            "max": {"type": "integer", "minimum": 1},
                            "idle_timeout": {"type": "integer", "minimum": 0}
                        },
                        "required": ["min", "max"]
                    }
                },
                "required": ["type", "host", "port", "name"]
            },
            "logging": {
                "type": "object",
                "properties": {
                    "level": {"enum": ["trace", "debug", "info", "warn", "error", "fatal"]},
                    "format": {"enum": ["json", "text", "structured"]},
                    "outputs": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "properties": {
                                "type": {"enum": ["console", "file", "syslog", "http"]},
                                "config": {
                                    "type": "object",
                                    "patternProperties": {
                                        ".*": {
                                            "oneOf": [
                                                {"type": "string"},
                                                {"type": "integer"},
                                                {"type": "boolean"}
                                            ]
                                        }
                                    }
                                }
                            },
                            "required": ["type"]
                        }
                    }
                },
                "required": ["level", "format", "outputs"]
            },
            "features": {
                "type": "object",
                "patternProperties": {
                    "^[a-z][a-z0-9_]*[a-z0-9]$": {
                        "type": "object",
                        "properties": {
                            "enabled": {"type": "boolean"},
                            "config": {"type": "object"},
                            "rollout": {
                                "type": "object",
                                "properties": {
                                    "percentage": {"type": "number", "minimum": 0, "maximum": 100},
                                    "audiences": {
                                        "type": "array",
                                        "items": {"type": "string"}
                                    }
                                }
                            }
                        },
                        "required": ["enabled"]
                    }
                }
            }
        },
        "required": ["service", "database", "logging"]
    });

    let start = std::time::Instant::now();
    let result = TctCompiler::new(complex_schema.clone(), "MicroserviceConfig")
        .unwrap()
        .compile();
    let generation_time = start.elapsed();

    if let Err(e) = &result {
        panic!("Complex microservice schema generation failed: {}", e);
    }
    assert!(
        result.is_ok(),
        "Complex microservice schema generation failed"
    );

    let grammar = result.unwrap();
    let start = std::time::Instant::now();
    let code = grammar.generate_code();
    let code_gen_time = start.elapsed();

    assert!(code.is_ok(), "Complex microservice code generation failed");

    let generated_code = code.unwrap();
    assert!(!generated_code.is_empty());
    println!("✅ Complex real-world performance test passed");
    println!("   Schema generation: {:?}", generation_time);
    println!("   Code generation: {:?}", code_gen_time);
    println!(
        "   Generated code length: {} characters",
        generated_code.len()
    );
}

/// Test configuration file patterns from popular tools
#[test]
fn test_real_world_schemas_extended_configuration_file_patterns() {
    let configs = [
        // Docker Compose style
        json!({
            "title": "DockerCompose",
            "type": "object",
            "properties": {
                "version": {"type": "string"},
                "services": {
                    "type": "object",
                    "patternProperties": {
                        "^[a-zA-Z0-9][a-zA-Z0-9_.-]*$": {
                            "type": "object",
                            "properties": {
                                "image": {"type": "string"},
                                "ports": {
                                    "type": "array",
                                    "items": {"type": "string"}
                                },
                                "environment": {
                                    "oneOf": [
                                        {
                                            "type": "array",
                                            "items": {"type": "string"}
                                        },
                                        {
                                            "type": "object",
                                            "patternProperties": {
                                                ".*": {"type": "string"}
                                            }
                                        }
                                    ]
                                },
                                "volumes": {
                                    "type": "array",
                                    "items": {"type": "string"}
                                }
                            }
                        }
                    }
                }
            },
            "required": ["version", "services"]
        }),
        // CI/CD Pipeline style
        json!({
            "title": "CIPipeline",
            "type": "object",
            "properties": {
                "stages": {
                    "type": "array",
                    "items": {
                        "type": "object",
                        "properties": {
                            "name": {"type": "string"},
                            "stage": {"enum": ["build", "test", "deploy", "cleanup"]},
                            "script": {
                                "type": "array",
                                "items": {"type": "string"}
                            },
                            "artifacts": {
                                "type": "object",
                                "properties": {
                                    "paths": {
                                        "type": "array",
                                        "items": {"type": "string"}
                                    },
                                    "expire_in": {"type": "string"}
                                }
                            },
                            "only": {
                                "type": "array",
                                "items": {"type": "string"}
                            }
                        },
                        "required": ["name", "stage", "script"]
                    }
                },
                "variables": {
                    "type": "object",
                    "patternProperties": {
                        "^[A-Z][A-Z0-9_]*$": {"type": "string"}
                    }
                }
            },
            "required": ["stages"]
        }),
    ];

    for (i, schema) in configs.iter().enumerate() {
        let default_title = format!("Config{}", i);
        let title = schema["title"].as_str().unwrap_or(&default_title);
        let result = TctCompiler::new(schema.clone(), title).unwrap().compile();

        assert!(
            result.is_ok(),
            "Configuration schema {} generation failed",
            title
        );

        let grammar = result.unwrap();
        let code = grammar.generate_code();
        assert!(
            code.is_ok(),
            "Configuration schema {} code generation failed",
            title
        );

        println!("✅ Configuration pattern test {} passed: {}", i, title);
    }
}
