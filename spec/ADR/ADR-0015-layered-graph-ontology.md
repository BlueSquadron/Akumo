# ADR-0015: Layered attack-graph ontology

- **Status:** Accepted
- **Date:** 2026-09-22
- **Ref:** SPEC-D4 · specification §2.2, §5.7

## Context
The graph is what the planner reasons over and what technique preconditions/effects reference.
A rich fixed canon risks forcing one cloud's concepts on others; a minimal core leaves the
planner without stable semantics.

## Decision
Use a **layered** ontology: a compact, provider-neutral **attack-core** (nodes: Principal,
Credential, Resource, PermissionSet, ExternalIdentity; edges incl. derived CAN_ACCESS,
CAN_ESCALATE_TO) that the planner traverses, **plus** a provider-attributed **inventory** layer
hanging off nodes. This is how BloodHound/Cartography scale.

## Consequences
- (+) Strong, stable planner semantics; stays provider-neutral; avoids a bloated premature canon.
- (−) Requires drawing the attack-core / inventory line well.

## Alternatives considered
- **Rich canonical ontology** — rejected: high upfront cost, risks misfitting future providers.
- **Minimal core + provider types** — rejected: no stable semantics; weak path-finding.
