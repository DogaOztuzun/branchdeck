---
name: bmad-story-cycle
description: 'Full epic development cycle: all stories implemented, verified, reviewed, and merged in sequence. Use when user says "dev epic N", "run story cycle", "implement epic [N]", or "story cycle [N.M]" to resume from a specific story.'
---

# Epic Development Cycle

## Overview

Automated epic development pipeline with complexity-aware story execution. Scores each story (simple/medium/complex), tags architecture level, and adjusts the approach: simple stories get oneshot implementation, complex stories get detailed context creation + party-mode review. Full epic party-mode review at completion.

**Input:** Epic number (e.g., `1`, `epic 6`) or story to resume from (e.g., `1.3`)
**Output:** Complete epic branch with all stories implemented, verified, reviewed, and merged. Ready for PR to main.

**Key features:**
- Complexity scoring: simple (auto-merge) / medium (full flow) / complex (create-story + party review)
- Architecture-level context: core, integration, frontend, infrastructure, standalone
- Party-mode review: after complex stories + always after full epic
- Parallel execution: run independent epics in separate terminals

## On Activation

Follow the instructions in ./workflow.md.
