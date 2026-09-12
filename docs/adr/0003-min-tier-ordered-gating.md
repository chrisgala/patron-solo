# 0003 — Ordered tiers with minimum-tier gating

Date: 2026-09-12 · Status: accepted

## Context

Patreon supports two gating models: hierarchical ("$X and above") and per-tier selection (a post lists exactly which tiers see it, allowing non-nested perks like an audio-only tier). They imply different schemas: a single `min_tier_level` column vs. a post↔tier join table.

## Decision

Tiers are strictly ordered by an integer `level`; a Post carries an optional `min_tier_level`. Access requires an active subscription whose tier level ≥ the post's minimum. Posts reference the *level*, not the tier row, so tiers can be renamed/repriced/replaced without orphaning posts.

## Consequences

- Simple mental model and one-column schema; entitlement checks are a single integer comparison.
- Non-hierarchical perk structures (parallel tiers with disjoint content) are impossible without a schema change (join table) — accepted, revisit only with real demand.
- Deleting a tier leaves posts gated at a level no active tier may hold; handlers validate min_tier_level against existing tiers and the UI warns.
