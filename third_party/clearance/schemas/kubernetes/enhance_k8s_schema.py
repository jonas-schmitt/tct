#!/usr/bin/env python3
"""
Enhance Kubernetes schema with proper enum constraints.

Many K8s fields are declared as "type": "string" but have documented
valid values. This script adds enum constraints to enable TCT's
EnumIndexOptimization to work on these fields.
"""

import json
import sys
from pathlib import Path

# Fields to enhance with their enum values
# Format: field_name -> (enum_values, description_pattern_to_match)
ENUM_ENHANCEMENTS = {
    "imagePullPolicy": {
        "values": ["Always", "Never", "IfNotPresent"],
        "pattern": "One of Always, Never, IfNotPresent"
    },
    "restartPolicy": {
        "values": ["Always", "OnFailure", "Never"],
        "pattern": "One of Always, OnFailure, Never"
    },
    "preemptionPolicy": {
        "values": ["Never", "PreemptLowerPriority"],
        "pattern": "One of Never, PreemptLowerPriority"
    },
    "protocol": {
        "values": ["TCP", "UDP", "SCTP"],
        "pattern": "Must be UDP, TCP, or SCTP"
    },
    "dnsPolicy": {
        "values": ["Default", "ClusterFirst", "ClusterFirstWithHostNet", "None"],
        "pattern": "DNS"  # Broad match - check description manually
    },
    "effect": {
        "values": ["NoSchedule", "PreferNoSchedule", "NoExecute"],
        "pattern": "effect"  # For taints/tolerations
    },
    "pathType": {
        "values": ["Exact", "Prefix", "ImplementationSpecific"],
        "pattern": "pathType"
    },
}

# Service type is special - only apply to ServiceSpec
SERVICE_TYPE_VALUES = ["ClusterIP", "NodePort", "LoadBalancer", "ExternalName"]


def enhance_schema(schema: dict) -> tuple[dict, dict]:
    """
    Add enum constraints to known string fields.
    Returns (enhanced_schema, stats).
    """
    stats = {field: 0 for field in ENUM_ENHANCEMENTS}
    stats["serviceType"] = 0

    def process_properties(props: dict, parent_def: str = "") -> None:
        for field_name, field_schema in props.items():
            if not isinstance(field_schema, dict):
                continue

            # Skip if already has enum
            if "enum" in field_schema:
                continue

            # Only process string fields
            if field_schema.get("type") != "string":
                continue

            # Check for known enum fields
            if field_name in ENUM_ENHANCEMENTS:
                enhancement = ENUM_ENHANCEMENTS[field_name]
                desc = field_schema.get("description", "")
                # Verify the field matches our expected pattern
                if enhancement["pattern"].lower() in desc.lower() or field_name in ["protocol", "effect", "pathType"]:
                    field_schema["enum"] = enhancement["values"]
                    stats[field_name] += 1

            # Special case: Service type field
            if field_name == "type" and "ServiceSpec" in parent_def:
                field_schema["enum"] = SERVICE_TYPE_VALUES
                stats["serviceType"] += 1

    # Process all definitions
    definitions = schema.get("definitions", {})
    for def_name, def_schema in definitions.items():
        if not isinstance(def_schema, dict):
            continue

        props = def_schema.get("properties", {})
        if props:
            process_properties(props, def_name)

    return schema, stats


def main():
    input_path = Path("schemas/popular/kubernetes-original.json")
    output_path = Path("schemas/popular/kubernetes.json")

    if len(sys.argv) > 1:
        input_path = Path(sys.argv[1])
    if len(sys.argv) > 2:
        output_path = Path(sys.argv[2])

    print(f"Loading schema from {input_path}...")
    with open(input_path) as f:
        schema = json.load(f)

    print("Enhancing schema with enum constraints...")
    enhanced, stats = enhance_schema(schema)

    # Update schema metadata
    enhanced["$id"] = "https://tct.dev/schemas/kubernetes-v1.json"
    enhanced["title"] = "Kubernetes Manifest (with Enum Constraints)"
    enhanced["description"] = (
        "Union schema for stable Kubernetes v1 resource types with enum constraints "
        "for fields like imagePullPolicy, protocol, etc. for improved tokenization efficiency."
    )

    # Ensure output directory exists
    output_path.parent.mkdir(parents=True, exist_ok=True)

    print(f"Writing enhanced schema to {output_path}...")
    with open(output_path, "w") as f:
        json.dump(enhanced, f, indent=2)

    print("\nEnhancement statistics:")
    total = 0
    for field, count in sorted(stats.items()):
        if count > 0:
            print(f"  {field}: {count} fields enhanced")
            total += count
    print(f"  Total: {total} fields enhanced")

    return 0


if __name__ == "__main__":
    sys.exit(main())
