# Union Decoding Test Schemas

These synthetic schemas are regression cases for `oneOf`/`anyOf` unions whose
variants have different JSON types. Each instance must decode back to the
variant it was encoded from.

## Test Schemas

### `string_vs_object_union.json`
**Tests**: String/object union disambiguation with boolean field confusion
**Problem**: The byte tokens of the string variant could be read as the fields of the object variant (for example, as its boolean field)
**Solution**: The encoded variant selection, not the payload shape, decides which variant is decoded

**Test Cases**:
- `string_version.json`: `{"version": "0.1.0"}` - Should decode as string variant
- `object_version.json`: `{"version": {"workspace": true}}` - Should decode as object variant

### `primitive_confusion.json`
**Tests**: bool/number/string primitive type disambiguation
**Problem**: Primitive tokens can be misinterpreted across types without schema context
**Solution**: Schema-aware primitive routing based on expected structure

**Test Cases**:
- `bool_value.json`: `{"value": true}` - Should decode as boolean variant
- `number_value.json`: `{"value": 42}` - Should decode as number variant
- `string_value.json`: `{"value": "hello"}` - Should decode as string variant

### `array_vs_object.json`
**Tests**: Array/object structure disambiguation
**Problem**: Array and object tokens with similar patterns could be confused
**Solution**: Schema-aware structure analysis distinguishes arrays from objects

**Test Cases**:
- `array_data.json`: `{"data": ["item1", "item2", "item3"]}` - Should decode as array variant
- `object_data.json`: `{"data": {"items": ["item1", "item2", "item3"]}}` - Should decode as object variant

### `optional_struct_false_positive.json`
**Tests**: All-optional struct false positive elimination
**Problem**: Structs with all optional fields can deserialize from any JSON by setting all fields to None
**Solution**: Schema-aware variant selection based on actual token structure

**Test Cases**:
- `string_config.json`: `{"config": "simple_config_string"}` - Should decode as string variant
- `object_config.json`: `{"config": {"option1": "value1", "option2": true}}` - Should decode as object variant

## Expected Behavior

Every test case round-trips: decoding returns the same variant and value that
was encoded, and no other variant accepts the instance.

## Validation

```bash
cargo test --test synthetic_schema_test test_decoder_string_vs_object_union -- --ignored --exact
cargo test --test synthetic_schema_test test_decoder_primitive_confusion -- --ignored --exact
cargo test --test synthetic_schema_test test_decoder_array_vs_object -- --ignored --exact
cargo test --test synthetic_schema_test test_decoder_optional_struct_false_positive -- --ignored --exact
```

All tests should pass with the new universal schema-aware decoding architecture.