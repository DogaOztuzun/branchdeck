use log::{debug, error, info, warn};

use crate::models::workflow::{
    DispatchEffect, DispatchPlan, TriggerContext, TriggerEvent, WorkflowDef,
};
use crate::services::workflow::WorkflowRegistry;

/// Build a dispatch plan for an incoming trigger event.
///
/// Matches the event against the registry, then produces effects for the first
/// matching workflow: create worktree, write context, deploy skill, enqueue run.
///
/// Returns a plan with `LogNoMatch` effect if no workflow matches.
#[must_use]
pub fn plan_dispatch(
    registry: &WorkflowRegistry,
    event: &TriggerEvent,
    repo_path: &str,
) -> DispatchPlan {
    let mut matches = registry.match_workflows(event);
    matches.sort_by_key(|d| d.config.name.clone());

    if matches.is_empty() {
        debug!("No workflow matched trigger event kind={}", event.kind);
        return DispatchPlan {
            workflow_name: String::new(),
            effects: vec![DispatchEffect::LogNoMatch {
                event_kind: event.kind,
                detail: format!("No workflow matched for {}", event.kind),
            }],
        };
    }

    if matches.len() > 1 {
        warn!(
            "Multiple workflows matched trigger event kind={}, using first: '{}'",
            event.kind, matches[0].config.name
        );
    }

    let def = matches[0];
    build_dispatch_plan(def, event, repo_path)
}

/// Build the full dispatch plan for a matched workflow + event.
/// Pure function: returns effects, no I/O.
fn build_dispatch_plan(
    def: &WorkflowDef,
    event: &TriggerEvent,
    repo_path: &str,
) -> DispatchPlan {
    let workflow_name = def.config.name.clone();
    let mut effects = Vec::new();

    let (worktree_branch, worktree_suffix) = derive_worktree_info(event, &workflow_name);
    let safe_suffix = crate::services::git::sanitize_worktree_name(&worktree_suffix);
    let worktree_path = format!("{repo_path}/.branchdeck/worktrees/{safe_suffix}");

    // 1. Create worktree
    effects.push(DispatchEffect::CreateWorktree {
        repo_path: repo_path.to_string(),
        branch: worktree_branch,
        worktree_path: worktree_path.clone(),
    });

    // 2. Write context from event data
    let context_json = serde_json::to_string_pretty(&event.context).unwrap_or_else(|e| {
        error!("Failed to serialize trigger context: {e}");
        String::new()
    });
    effects.push(DispatchEffect::WriteContext {
        worktree_path: worktree_path.clone(),
        context_file: ".branchdeck/context.json".to_string(),
        content: context_json,
    });

    // 3. Deploy skill (the workflow's prompt body)
    if !def.prompt.trim().is_empty() {
        effects.push(DispatchEffect::DeploySkill {
            worktree_path: worktree_path.clone(),
            skill_content: def.prompt.clone(),
        });
    }

    // 4. Enqueue the run with cost cap and directory confinement
    let max_budget_usd = def.config.agent.as_ref().and_then(|a| a.max_budget_usd);
    let allowed_directories = vec![worktree_path.clone()];

    effects.push(DispatchEffect::EnqueueRun {
        worktree_path: worktree_path.clone(),
        task_path: format!("{worktree_path}/.branchdeck/task.md"),
        max_budget_usd,
        allowed_directories,
    });

    // 5. Emit lifecycle event: dispatched
    let status_name = def
        .config
        .lifecycle
        .as_ref()
        .and_then(|l| l.dispatched.clone())
        .unwrap_or_else(|| "dispatched".to_string());

    effects.push(DispatchEffect::EmitWorkflowEvent {
        workflow_name: workflow_name.clone(),
        status: status_name,
        detail: format!("Workflow '{workflow_name}' dispatched"),
    });

    info!("Planned dispatch for workflow '{workflow_name}' at {worktree_path}");

    DispatchPlan {
        workflow_name,
        effects,
    }
}

/// Derive worktree branch name and path suffix from trigger event context.
fn derive_worktree_info(event: &TriggerEvent, workflow_name: &str) -> (String, String) {
    match &event.context {
        TriggerContext::GithubIssue { number, .. } => (
            format!("workflow/{workflow_name}-issue-{number}"),
            format!("{workflow_name}-issue-{number}"),
        ),
        TriggerContext::GithubPr {
            number, branch, ..
        } => (branch.clone(), format!("{workflow_name}-pr-{number}")),
        TriggerContext::Manual {
            workflow_name: name,
            ..
        } => (format!("workflow/{name}"), format!("{name}-manual")),
        TriggerContext::PostMerge { pr_number, .. } => (
            format!("workflow/{workflow_name}-post-{pr_number}"),
            format!("{workflow_name}-post-{pr_number}"),
        ),
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::disallowed_methods)]
mod tests {
    use super::*;
    use crate::models::workflow::*;
    use crate::services::workflow::WorkflowRegistry;
    use std::collections::HashMap;

    fn make_issue_workflow() -> &'static str {
        "---\n\
         name: implement-issue\n\
         description: Implement GitHub issues\n\
         tracker:\n\
         \x20 kind: github-issue\n\
         \x20 filter:\n\
         \x20   label: \"agent:implement\"\n\
         agent:\n\
         \x20 max_budget_usd: 5.0\n\
         outcomes:\n\
         \x20 - name: pr-created\n\
         \x20   detect: pr-created\n\
         \x20   next: complete\n\
         ---\n\
         Implement the issue described in context.json.\n"
    }

    fn make_pr_workflow() -> &'static str {
        "---\n\
         name: pr-shepherd\n\
         description: Fix failing PRs\n\
         tracker:\n\
         \x20 kind: github-pr\n\
         \x20 filter:\n\
         \x20   ci_status: \"FAILURE\"\n\
         outcomes:\n\
         \x20 - name: ci-passing\n\
         \x20   detect: ci-passing\n\
         \x20   next: complete\n\
         ---\n\
         Fix the CI failures.\n"
    }

    fn make_issue_event(label: &str) -> TriggerEvent {
        TriggerEvent {
            kind: TrackerKind::GithubIssue,
            context: TriggerContext::GithubIssue {
                repo: "owner/repo".to_string(),
                number: 42,
                title: "Fix the bug".to_string(),
                body: Some("The login button is broken on the dashboard.".to_string()),
                labels: vec![label.to_string()],
            },
        }
    }

    fn make_pr_event(ci_status: &str) -> TriggerEvent {
        TriggerEvent {
            kind: TrackerKind::GithubPr,
            context: TriggerContext::GithubPr {
                repo: "owner/repo".to_string(),
                number: 7,
                branch: "fix/thing".to_string(),
                base_branch: "main".to_string(),
                ci_status: Some(ci_status.to_string()),
                review_decision: None,
            },
        }
    }

    fn build_registry(workflow_mds: &[&str]) -> WorkflowRegistry {
        use tempfile::TempDir;

        let tmp = TempDir::new().unwrap();
        for (i, md) in workflow_mds.iter().enumerate() {
            let dir = tmp.path().join(format!("wf-{i}"));
            std::fs::create_dir_all(&dir).unwrap();
            std::fs::write(dir.join("WORKFLOW.md"), md).unwrap();
        }
        WorkflowRegistry::scan(&[tmp.path().to_path_buf()])
    }

    #[test]
    fn plan_dispatch_matches_issue_workflow() {
        let registry = build_registry(&[make_issue_workflow()]);
        let event = make_issue_event("agent:implement");

        let plan = plan_dispatch(&registry, &event, "/tmp/repo");
        assert_eq!(plan.workflow_name, "implement-issue");
        assert!(plan.effects.len() >= 4);

        assert!(plan.effects.iter().any(|e| matches!(
            e,
            DispatchEffect::CreateWorktree { worktree_path, .. }
            if worktree_path.contains("implement-issue-issue-42")
        )));

        assert!(plan.effects.iter().any(|e| matches!(
            e,
            DispatchEffect::EnqueueRun { max_budget_usd: Some(budget), .. }
            if (*budget - 5.0).abs() < f64::EPSILON
        )));

        assert!(plan.effects.iter().any(|e| matches!(
            e,
            DispatchEffect::EnqueueRun { allowed_directories, .. }
            if allowed_directories.len() == 1
        )));
    }

    #[test]
    fn plan_dispatch_issue_context_includes_body() {
        let registry = build_registry(&[make_issue_workflow()]);
        let event = make_issue_event("agent:implement");

        let plan = plan_dispatch(&registry, &event, "/tmp/repo");

        let context_effect = plan.effects.iter().find(|e| {
            matches!(
                e,
                DispatchEffect::WriteContext {
                    context_file, ..
                } if context_file == ".branchdeck/context.json"
            )
        });
        assert!(context_effect.is_some(), "WriteContext effect must exist");

        if let Some(DispatchEffect::WriteContext { content, .. }) = context_effect {
            assert!(content.contains("Fix the bug"), "context must include title");
            assert!(
                content.contains("login button is broken"),
                "context must include body"
            );
            assert!(
                content.contains("agent:implement"),
                "context must include labels"
            );
            assert!(
                content.contains("owner/repo"),
                "context must include repo"
            );
        }
    }

    #[test]
    fn plan_dispatch_no_match_produces_log_effect() {
        let registry = build_registry(&[make_issue_workflow()]);
        let event = make_issue_event("unrelated-label");

        let plan = plan_dispatch(&registry, &event, "/tmp/repo");
        assert!(plan
            .effects
            .iter()
            .any(|e| matches!(e, DispatchEffect::LogNoMatch { .. })));
    }

    #[test]
    fn plan_dispatch_pr_workflow_matches_ci_failure() {
        let registry = build_registry(&[make_pr_workflow()]);
        let event = make_pr_event("FAILURE");

        let plan = plan_dispatch(&registry, &event, "/tmp/repo");
        assert_eq!(plan.workflow_name, "pr-shepherd");
    }

    #[test]
    fn plan_dispatch_pr_workflow_no_match_on_success() {
        let registry = build_registry(&[make_pr_workflow()]);
        let event = make_pr_event("SUCCESS");

        let plan = plan_dispatch(&registry, &event, "/tmp/repo");
        assert!(plan
            .effects
            .iter()
            .any(|e| matches!(e, DispatchEffect::LogNoMatch { .. })));
    }

    #[test]
    fn plan_dispatch_manual_trigger_wrong_kind() {
        let registry = build_registry(&[make_issue_workflow()]);
        let event = TriggerEvent {
            kind: TrackerKind::Manual,
            context: TriggerContext::Manual {
                workflow_name: "implement-issue".to_string(),
                params: HashMap::new(),
            },
        };

        let plan = plan_dispatch(&registry, &event, "/tmp/repo");
        assert!(plan
            .effects
            .iter()
            .any(|e| matches!(e, DispatchEffect::LogNoMatch { .. })));
    }

    #[test]
    fn plan_dispatch_manual_trigger_with_manual_workflow() {
        let md = "---\n\
                   name: sat-cycle\n\
                   description: Run SAT manually\n\
                   tracker:\n\
                   \x20 kind: manual\n\
                   outcomes:\n\
                   \x20 - name: scores-written\n\
                   \x20   detect: file-exists\n\
                   \x20   path: sat/scores.json\n\
                   \x20   next: complete\n\
                   ---\n\
                   Run SAT cycle.\n";
        let registry = build_registry(&[md]);
        let event = TriggerEvent {
            kind: TrackerKind::Manual,
            context: TriggerContext::Manual {
                workflow_name: "sat-cycle".to_string(),
                params: HashMap::new(),
            },
        };

        let plan = plan_dispatch(&registry, &event, "/tmp/repo");
        assert_eq!(plan.workflow_name, "sat-cycle");
    }
}
