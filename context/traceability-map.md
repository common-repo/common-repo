# Traceability Map

This document provides a consolidated reference mapping each component to its corresponding plan and design documentation.

## Core Components

| Component | Plan Reference | Design Reference |
|-----------|----------------|------------------|
| Overall Implementation | [Implementation Strategy](completed/implementation-plan.md#implementation-strategy) | [Execution Model](design.md#execution-model) |

## Layer 0 - Foundation

| Component | Plan Reference | Design Reference |
|-----------|----------------|------------------|
| Layer 0 Overview | [Layer 0 Foundation](completed/implementation-plan.md#layer-0-foundation-no-dependencies) | [Phase 2: Processing Individual Repos](design.md#phase-2-processing-individual-repos) |
| 0.1 Configuration Schema & Parsing | [0.1 Configuration Schema](completed/implementation-plan.md#01-configuration-schema--parsing) | [Phase 1: Discovery and Cloning](design.md#phase-1-discovery-and-cloning) |
| 0.2 In-Memory Filesystem | [0.2 In-Memory Filesystem](completed/implementation-plan.md#02-in-memory-filesystem) | [Core Concepts](design.md#core-concepts) |
| 0.3 Error Handling | [0.3 Error Handling](completed/implementation-plan.md#03-error-handling) | [Error Handling](design.md#error-handling) |

## Layer 1 - Core Utilities

| Component | Plan Reference | Design Reference |
|-----------|----------------|------------------|
| Layer 1 Overview | [Layer 1 Core Utilities](completed/implementation-plan.md#layer-1-core-utilities-depends-on-layer-0) | [Execution Model](design.md#execution-model) |
| 1.2 Path Operations | [1.2 Path Operations](completed/implementation-plan.md#12-path-operations) | [Operator Implementation Details](design.md#operator-implementation-details) |
| 1.3 Repository Cache | [1.3 Repository Cache](completed/implementation-plan.md#13-repository-cache) | [Caching Strategy](design.md#caching-strategy) |
| 1.4 Repository Manager | [1.2 Repository Manager](completed/implementation-plan.md#12-repository-manager) | [Phase 1: Discovery and Cloning](design.md#phase-1-discovery-and-cloning) |

## Layer 2 - Operators

| Component | Plan Reference | Design Reference |
|-----------|----------------|------------------|
| Layer 2 Overview | [Layer 2 Operators](completed/implementation-plan.md#layer-2-operators-depends-on-layers-0-1) | [Operator Implementation Details](design.md#operator-implementation-details) |
| 2.1 Repo Operator | [2.1 Repo Operator](completed/implementation-plan.md#21-repo-operator) | [Operator: repo](design.md#repo) |
| 2.2 Basic File Operators | [2.2 Basic File Operators](completed/implementation-plan.md#22-basic-file-operators) | [Operator Implementation Details](design.md#operator-implementation-details) |
| 2.3 Template Operators | [2.3 Template Operators](completed/implementation-plan.md#23-template-operators) | [Operator: template](design.md#template) |
| 2.4 Merge Operators | [2.4 Merge Operators](completed/implementation-plan.md#24-merge-operators) | [Fragment Merge Operators](design.md#fragment-merge-operators) |

## Layer 3 - Phases & Orchestration

| Component | Plan Reference | Design Reference |
|-----------|----------------|------------------|
| Layer 3 Overview | [Layer 3 Phases](completed/implementation-plan.md#layer-3-phases-depends-on-layers-0-2) | [Execution Model](design.md#execution-model) |
| Phase 1: Discovery & Cloning | [3.1 Phase 1](completed/implementation-plan.md#31-phase-1-discovery-and-cloning) | [Phase 1: Discovery and Cloning](design.md#phase-1-discovery-and-cloning) |
| Phase Orchestrator | [Layer 3 Phases](completed/implementation-plan.md#layer-3-phases-depends-on-layers-0-2) | [High-Level Flow](design.md#high-level-flow) |

## Layer 3.5 - Version Detection

| Component | Plan Reference | Design Reference |
|-----------|----------------|------------------|
| Version Detection | [Layer 3.5 Version Detection](completed/implementation-plan.md#layer-35-version-detection-depends-on-layers-0-1) | [Version Detection and Updates](design.md#version-detection-and-updates) |

## Layer 4 - CLI & Orchestration

| Component | Plan Reference | Design Reference |
|-----------|----------------|------------------|
| Layer 4 Overview | [Layer 4 CLI & Orchestration](completed/implementation-plan.md#layer-4-cli--orchestration-depends-on-all-layers) | [CLI Design](design.md#cli-design) |
| 4.1 CLI Interface | [4.1 CLI Interface](completed/implementation-plan.md#41-cli-interface) | [CLI Design](design.md#cli-design) |
| 4.2 Main Orchestrator | [4.2 Main Orchestrator](completed/implementation-plan.md#42-main-orchestrator) | [CLI Design](design.md#cli-design) |

## Testing & Documentation

| Component | Plan Reference | Design Reference |
|-----------|----------------|------------------|
| Testing Strategy | [Testing Strategy](completed/implementation-plan.md#testing-strategy) | [Testing Strategy](design.md#testing-strategy) |
| Integration Tests | [Testing Strategy](completed/implementation-plan.md#testing-strategy) | [Testing Strategy](design.md#testing-strategy) |
| Documentation | [Implementation Strategy](completed/implementation-plan.md#implementation-strategy) | [Testing Strategy](design.md#testing-strategy) |

## Dependencies

| Component | Plan Reference | Design Reference |
|-----------|----------------|------------------|
| Essential Crates | [Dependencies Summary](completed/implementation-plan.md#dependencies-summary) | [Execution Model](design.md#execution-model) |

## Notes

- All plan references are relative to `context/completed/implementation-plan.md` (archived after completion)
- All design references are relative to `context/design.md`
- Feature implementation status is archived in `context/completed/feature-status.json`
