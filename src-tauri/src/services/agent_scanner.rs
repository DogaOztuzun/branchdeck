use crate::error::AppError;
use crate::models::agent::AgentDefinition;
use log::{debug, warn};
use std::path::Path;

/// Scans `.claude/agents/*.md` files in the given repo and returns parsed agent definitions.
///
/// Files that fail to parse are logged and skipped.
pub fn scan_agent_definitions(repo_path: &str) -> Result<Vec<AgentDefinition>, AppError> {
    let agents_dir = Path::new(repo_path).join(".claude/agents");

    if !agents_dir.is_dir() {
        debug!("No .claude/agents/ directory found in {repo_path:?}");
        return Ok(Vec::new());
    }

    let entries = std::fs::read_dir(&agents_dir).map_err(|e| {
        AppError::Agent(format!(
            "Failed to read agents directory {}: {e}",
            agents_dir.display()
        ))
    })?;

    let mut definitions = Vec::new();

    for entry in entries {
        let entry =
            entry.map_err(|e| AppError::Agent(format!("Failed to read directory entry: {e}")))?;

        let path = entry.path();

        if path.extension().and_then(std::ffi::OsStr::to_str) != Some("md") {
            continue;
        }

        match parse_agent_file(&path) {
            Ok(def) => {
                debug!(
                    "Parsed agent definition {:?} from {}",
                    def.name,
                    path.display()
                );
                definitions.push(def);
            }
            Err(e) => {
                warn!("Skipping agent file {}: {e}", path.display());
            }
        }
    }

    debug!(
        "Scanned {} agent definitions from {repo_path:?}",
        definitions.len()
    );
    Ok(definitions)
}

fn parse_agent_file(path: &Path) -> Result<AgentDefinition, AppError> {
    let contents = std::fs::read_to_string(path)
        .map_err(|e| AppError::Agent(format!("Failed to read {}: {e}", path.display())))?;

    let frontmatter = extract_frontmatter(&contents, path)?;

    let mut name: Option<String> = None;
    let mut description = String::new();
    let mut model: Option<String> = None;
    let mut tools: Vec<String> = Vec::new();
    let mut permission_mode: Option<String> = None;

    for line in frontmatter.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        let Some((key, value)) = line.split_once(':') else {
            continue;
        };

        let key = key.trim();
        let value = value.trim();

        match key {
            "name" => name = Some(strip_quotes(value)),
            "description" => description = strip_quotes(value),
            "model" => model = Some(strip_quotes(value)),
            "tools" => tools = parse_bracket_array(value),
            "permission_mode" => permission_mode = Some(strip_quotes(value)),
            _ => {
                debug!("Unknown frontmatter key {key:?} in {}", path.display());
            }
        }
    }

    let Some(name) = name else {
        return Err(AppError::Agent(format!(
            "Missing required 'name' field in {}",
            path.display()
        )));
    };

    let file_path = path
        .canonicalize()
        .unwrap_or_else(|_| path.to_path_buf())
        .to_string_lossy()
        .into_owned();

    Ok(AgentDefinition {
        name,
        description,
        model,
        tools,
        permission_mode,
        file_path,
    })
}

fn extract_frontmatter<'a>(contents: &'a str, path: &Path) -> Result<&'a str, AppError> {
    let trimmed = contents.trim_start();

    let Some(rest) = trimmed.strip_prefix("---") else {
        return Err(AppError::Agent(format!(
            "No opening frontmatter delimiter in {}",
            path.display()
        )));
    };

    // Skip the rest of the opening delimiter line
    let rest = rest.trim_start_matches(|c: char| c != '\n');
    let rest = rest.strip_prefix('\n').unwrap_or(rest);

    let Some(end) = rest.find("\n---") else {
        return Err(AppError::Agent(format!(
            "No closing frontmatter delimiter in {}",
            path.display()
        )));
    };

    Ok(&rest[..end])
}

fn strip_quotes(s: &str) -> String {
    let s = s.trim();
    if (s.starts_with('"') && s.ends_with('"')) || (s.starts_with('\'') && s.ends_with('\'')) {
        s[1..s.len() - 1].to_owned()
    } else {
        s.to_owned()
    }
}

fn parse_bracket_array(value: &str) -> Vec<String> {
    let value = value.trim();

    let Some(inner) = value.strip_prefix('[').and_then(|v| v.strip_suffix(']')) else {
        // Not a bracket array — treat as single value
        let stripped = strip_quotes(value);
        if stripped.is_empty() {
            return Vec::new();
        }
        return vec![stripped];
    };

    inner
        .split(',')
        .map(|item| strip_quotes(item.trim()))
        .filter(|item| !item.is_empty())
        .collect()
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn test_strip_quotes() {
        assert_eq!(strip_quotes("\"hello\""), "hello");
        assert_eq!(strip_quotes("'hello'"), "hello");
        assert_eq!(strip_quotes("hello"), "hello");
        assert_eq!(strip_quotes("  \"spaced\"  "), "spaced");
    }

    #[test]
    fn test_parse_bracket_array() {
        let result = parse_bracket_array("[\"Read\", \"Write\", \"Edit\"]");
        assert_eq!(result, vec!["Read", "Write", "Edit"]);
    }

    #[test]
    fn test_parse_bracket_array_empty() {
        let result = parse_bracket_array("[]");
        assert!(result.is_empty());
    }

    #[test]
    fn test_parse_bracket_array_single_value() {
        let result = parse_bracket_array("\"Read\"");
        assert_eq!(result, vec!["Read"]);
    }

    #[test]
    fn test_extract_frontmatter_valid() {
        let contents = "---\nname: test\ndescription: A test\n---\n\nBody content here.";
        let path = Path::new("test.md");
        let fm = extract_frontmatter(contents, path).unwrap();
        assert_eq!(fm, "name: test\ndescription: A test");
    }

    #[test]
    fn test_extract_frontmatter_no_opener() {
        let contents = "name: test\n---\n";
        let path = Path::new("test.md");
        assert!(extract_frontmatter(contents, path).is_err());
    }

    #[test]
    fn test_parse_agent_file_full() {
        let dir = std::env::temp_dir().join("branchdeck_test_agent");
        let _ = std::fs::create_dir_all(&dir);
        let file = dir.join("researcher.md");
        std::fs::write(
            &file,
            "---\nname: researcher\ndescription: \"Research agent\"\nmodel: opus\ntools: [\"Read\", \"Grep\"]\npermission_mode: auto\n---\n\nSystem prompt here.",
        )
        .unwrap();

        let def = parse_agent_file(&file).unwrap();
        assert_eq!(def.name, "researcher");
        assert_eq!(def.description, "Research agent");
        assert_eq!(def.model.as_deref(), Some("opus"));
        assert_eq!(def.tools, vec!["Read", "Grep"]);
        assert_eq!(def.permission_mode.as_deref(), Some("auto"));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_parse_agent_file_missing_name() {
        let dir = std::env::temp_dir().join("branchdeck_test_agent_noname");
        let _ = std::fs::create_dir_all(&dir);
        let file = dir.join("bad.md");
        std::fs::write(&file, "---\ndescription: no name\n---\n").unwrap();

        assert!(parse_agent_file(&file).is_err());

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_scan_nonexistent_dir() {
        let result = scan_agent_definitions("/tmp/branchdeck_nonexistent_dir_12345");
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }
}
