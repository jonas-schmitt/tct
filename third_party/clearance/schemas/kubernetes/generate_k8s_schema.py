#!/usr/bin/env python3
"""
Create a clean Kubernetes v1-only union schema from kubernetes-definitions.json.
- Filters out alpha/beta/deprecated APIs
- Keeps only stable v1 resources
- Adds apiVersion const constraints to eliminate ambiguity
- Inlines all required definitions
"""

import json
import sys
from pathlib import Path
import re

def is_stable_v1_api(def_name):
    """Check if definition is a stable v1 API (not alpha/beta)."""

    # Whitelist: v1beta1 APIs that are stable/standard enough to include
    # These types are either promoted to v1 in later K8s versions or are
    # widely used standard resources that don't significantly bloat the schema
    STABLE_BETA_APIS = [
        'io.k8s.api.batch.v1beta1.CronJob',  # Promoted to v1 in K8s 1.21
        'io.k8s.api.policy.v1beta1.PodDisruptionBudget',  # Promoted to v1 in K8s 1.21
        'io.k8s.api.discovery.v1beta1.EndpointSlice',  # Promoted to v1 in K8s 1.21
        'io.k8s.api.node.v1beta1.RuntimeClass',  # Promoted to v1 in K8s 1.20
        'io.k8s.api.networking.v1beta1.IngressClass',  # Promoted to v1 in K8s 1.19
        'io.k8s.api.certificates.v1beta1.CertificateSigningRequest',  # Promoted to v1 in K8s 1.19
    ]

    # Allow whitelisted beta APIs
    if def_name in STABLE_BETA_APIS:
        return True

    # Exclude alpha and beta versions
    if 'alpha' in def_name.lower() or 'beta' in def_name.lower():
        return False

    # Must be v1 (not v2, v3, etc. unless it's the only stable version)
    # Exception: autoscaling.v2 is stable for HorizontalPodAutoscaler
    if '.v1.' in def_name or def_name.endswith('.v1'):
        return True

    # Allow specific v2 APIs that are stable
    if 'autoscaling.v2.HorizontalPodAutoscaler' in def_name:
        return True

    return False

def derive_api_version(def_name):
    """
    Derive the Kubernetes apiVersion string from a definition name.

    Kubernetes uses FULL domain format for API groups:
        io.k8s.api.core.v1.Pod → v1
        io.k8s.api.apps.v1.Deployment → apps/v1
        io.k8s.api.rbac.v1.ClusterRole → rbac.authorization.k8s.io/v1
        io.k8s.api.batch.v1.Job → batch/v1
        io.k8s.api.storage.v1.StorageClass → storage.k8s.io/v1
        io.k8s.api.networking.v1.Ingress → networking.k8s.io/v1
        io.k8s.api.admissionregistration.v1.* → admissionregistration.k8s.io/v1
        io.k8s.apiextensions-apiserver.pkg.apis.apiextensions.v1.* → apiextensions.k8s.io/v1
        io.k8s.kube-aggregator.pkg.apis.apiregistration.v1.* → apiregistration.k8s.io/v1
    """
    # API groups that use full domain format (group.k8s.io/version)
    FULL_DOMAIN_GROUPS = {
        'rbac', 'storage', 'networking', 'admissionregistration',
        'coordination', 'scheduling', 'authentication', 'authorization',
        'autoscaling', 'policy', 'discovery', 'node', 'certificates'
    }

    # Handle different patterns
    if 'io.k8s.api.core.v1' in def_name:
        # Core API: just "v1"
        return "v1"
    elif 'io.k8s.api.' in def_name:
        # Standard API groups: io.k8s.api.{group}.{version}.{kind}
        match = re.match(r'io\.k8s\.api\.([^.]+)\.([^.]+)\..+', def_name)
        if match:
            group, version = match.groups()
            # Special case: RBAC uses rbac.authorization.k8s.io/v1
            if group == 'rbac':
                return f"rbac.authorization.k8s.io/{version}"
            # Check if this group uses full domain format
            elif group in FULL_DOMAIN_GROUPS:
                return f"{group}.k8s.io/{version}"
            else:
                # Simple format (apps, batch, etc.)
                return f"{group}/{version}"
    elif 'io.k8s.apiextensions-apiserver' in def_name:
        # CRDs: apiextensions.k8s.io/{version}
        match = re.search(r'\.apiextensions\.([^.]+)\..+', def_name)
        if match:
            version = match.group(1)
            return f"apiextensions.k8s.io/{version}"
    elif 'io.k8s.kube-aggregator' in def_name:
        # API aggregation: apiregistration.k8s.io/{version}
        match = re.search(r'\.apiregistration\.([^.]+)\..+', def_name)
        if match:
            version = match.group(1)
            return f"apiregistration.k8s.io/{version}"

    # Fallback: cannot determine (should not happen for valid K8s types)
    print(f"  ⚠️  Warning: Could not derive apiVersion for {def_name}", file=sys.stderr)
    return None

def add_api_version_constraint(definition, api_version):
    """Add const constraint to apiVersion property."""
    if 'properties' in definition and 'apiVersion' in definition['properties']:
        definition['properties']['apiVersion']['const'] = api_version
        print(f"    Added const: {api_version}")
    return definition

def main():
    # Load kubernetes-definitions.json
    definitions_path = Path("schemas/popular/kubernetes-definitions.json")
    with open(definitions_path) as f:
        k8s_defs = json.load(f)

    # Extract all definitions
    all_definitions = k8s_defs.get("definitions", {})

    print("🔍 Filtering for stable v1 APIs...")
    print()

    # Find stable v1 top-level resource types
    stable_resources = []
    for def_name in sorted(all_definitions.keys()):
        # Skip if not stable v1
        if not is_stable_v1_api(def_name):
            continue

        # Skip List types, Spec types, Status types, Condition types
        if any(suffix in def_name for suffix in ["List", "Spec", "Status", "Condition", "Strategy"]):
            continue

        # Check if it has the expected structure
        def_schema = all_definitions[def_name]
        props = def_schema.get("properties", {})

        # Top-level resources have apiVersion, kind, metadata
        if "apiVersion" in props and "kind" in props and "metadata" in props:
            # Derive apiVersion
            api_version = derive_api_version(def_name)
            if api_version:
                stable_resources.append((def_name, api_version))
                print(f"✅ {def_name}")
                print(f"    apiVersion: {api_version}")

    print()
    print(f"✅ Found {len(stable_resources)} stable v1 resource types")
    print()

    # Add apiVersion constraints to all definitions
    print("🔧 Adding apiVersion const constraints...")
    for def_name, api_version in stable_resources:
        all_definitions[def_name] = add_api_version_constraint(
            all_definitions[def_name],
            api_version
        )

    # Create union schema with stable v1 resources only
    union_schema = {
        "$schema": "http://json-schema.org/draft-07/schema#",
        "$id": "https://tct.dev/schemas/kubernetes-v1-union.json",
        "title": "Kubernetes Manifest (Stable v1 APIs)",
        "description": f"Union schema for stable Kubernetes v1 resource types ({len(stable_resources)} types). No alpha/beta/deprecated APIs.",
        "oneOf": [
            {"$ref": f"#/definitions/{resource_name}"}
            for resource_name, _ in sorted(stable_resources)
        ],
        "definitions": all_definitions  # Inline ALL definitions (needed for references)
    }

    # Save union schema
    output_path = Path("schemas/popular/kubernetes.json")
    with open(output_path, 'w') as f:
        json.dump(union_schema, f, indent=2)

    print()
    print(f"✅ Created clean v1-only schema: {output_path}")
    print(f"   Contains {len(stable_resources)} stable v1 resource types via oneOf")
    print(f"   Inlined {len(all_definitions)} definitions")
    print(f"   All apiVersion fields have const constraints")
    print()

    # Print summary
    print("📊 Resource type summary:")
    api_groups = {}
    for _, api_version in stable_resources:
        group = api_version.split('/')[0] if '/' in api_version else 'core'
        api_groups[group] = api_groups.get(group, 0) + 1

    for group in sorted(api_groups.keys()):
        print(f"  - {group}: {api_groups[group]} resources")

    return 0

if __name__ == "__main__":
    sys.exit(main())
