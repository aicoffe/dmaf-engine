---
name: contract-testing
description: Contract testing. Consumer-driven contract verification of interfaces between services, contract management, CI gate integration. Use when the user asks for "contract testing", "interface compatibility verification", "Pact", or "inter-service interface changes".
---

# Contract Testing

Verifies the "structural agreements" between services (fields, types, status codes), not business correctness. Consumer and provider are tested independently; the two services do not need to run at the same time.

## Workflow

### 1. Consumer-driven flow
```
1. Consumer: write consumer tests for the fields it actually uses → generate the contract file
2. Publish the contract to the Pact Broker (version linked to the consumer's git commit)
3. Provider CI: pull the contract → verify it against the local service → write the results back to the Broker
4. Broker verdict: did the provider's change break any consumer?
```

### 2. Contract content rules
- Declare only the fields and status codes the consumer actually uses; do not over-constrain the unused parts
- Each contract scenario aligns with one business case (e.g. "query an existing claim, 200", "query a nonexistent one, 404")
- Message contracts are included the same way: the consumer generates the message schema contract for the fields it actually consumes (same principle as the HTTP contract — declare only what is used), evolving with the envelope version

### 3. Relationship with OpenAPI
- For external REST, openapi.yaml is the contract source; the provider verifies implementation consistency against the schema
- Pact and schema complement each other: the schema governs the full definition, the contract governs the part the consumer really uses

### 4. CI gate
- The provider must pass all consumers' contract verification before merging (can-i-deploy)
- Verification failure = breaking change: adjust per the compatibility rules (adding optional fields goes straight in; removing fields / changing types / changing error codes bumps major and goes through the deprecation process), or coordinate the migration with the consumer
- Contract change records are linked to the CHANGELOG

## Boundaries (not in this skill)

- Case design and TDD execution belong to `test-design` / `automated-testing`; this skill covers only inter-service contract verification and the merge gate

## Hard rules

- Interfaces between services must have contract tests; contract compatibility checks are part of the merge gate
- Bypassing the Broker with verbal "handshake agreements" on interface changes is forbidden

## Reference files

- `rules/standards/quality-standard.md` / `rules/standards/architecture-standard.md` — governing constraints
