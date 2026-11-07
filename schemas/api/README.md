# API Schemas

Real-world API specifications and service definitions.

## Contents

- **gRPC_API_Gateway__OpenAPI_Config.json** - gRPC API Gateway OpenAPI configuration
- **api-response.json** - Generic API response envelope used by the target schema gate

Optional API downloads without complete source/license evidence are listed in
`third_party/clearance/schemas/pending-schemas.json` and omitted from the public
source selection. The private preparation archive retains its original inputs.

## Common Issues

- OpenAPI specifications often mix with JSON Schema features
- Complex reference structures
- Custom vendor extensions not supported

## Testing

```bash
cargo test --test schema_compilation_test test_api_schemas
```
