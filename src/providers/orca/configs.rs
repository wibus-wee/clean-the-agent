use std::fs;
use std::path::{Path, PathBuf};

use jsonc_parser::ParseOptions;
use jsonc_parser::cst::{CstNode, CstObject, CstRootNode};

use crate::model::{ArtifactKind, FileFormat, Ownership, Safety, ScanReport};
use crate::provider::{HomeScope, Platform, ScanContext};

use super::support::{FindingSpec, RewriteSpec, add_path, add_rewrite, resolve_file};

struct ConfigSpec {
    relative_path: &'static str,
    product: &'static str,
    markers: &'static [&'static str],
    containers: &'static [&'static str],
    status_line: bool,
    format: FileFormat,
    owned_file: bool,
}

const SPECS: &[ConfigSpec] = &[
    spec(
        ".claude/settings.json",
        "Claude Code",
        CLAUDE,
        &["hooks"],
        true,
    ),
    spec(
        ".openclaude/settings.json",
        "OpenClaude",
        OPENCLAUDE,
        &["hooks"],
        false,
    ),
    spec(".codex/hooks.json", "Codex hooks", CODEX, &["hooks"], false),
    spec(".cursor/hooks.json", "Cursor", CURSOR, &["hooks"], false),
    spec(".gemini/settings.json", "Gemini", GEMINI, &["hooks"], false),
    spec(
        ".gemini/config/hooks.json",
        "Antigravity",
        ANTIGRAVITY,
        &["orca-status"],
        false,
    ),
    spec(
        ".factory/settings.json",
        "Droid/Factory",
        DROID,
        &["hooks"],
        false,
    ),
    spec(
        ".commandcode/settings.json",
        "Command Code",
        COMMAND_CODE,
        &["hooks"],
        false,
    ),
];

const fn spec(
    relative_path: &'static str,
    product: &'static str,
    markers: &'static [&'static str],
    containers: &'static [&'static str],
    status_line: bool,
) -> ConfigSpec {
    ConfigSpec {
        relative_path,
        product,
        markers,
        containers,
        status_line,
        format: FileFormat::Json,
        owned_file: false,
    }
}

const CLAUDE: &[&str] = &[
    "claude-hook.sh",
    "claude-hook.cmd",
    "claude-hook.ps1",
    "claude-statusline.sh",
    "claude-statusline.cmd",
    "claude-statusline.ps1",
];
const OPENCLAUDE: &[&str] = &[
    "openclaude-hook.sh",
    "openclaude-hook.cmd",
    "openclaude-hook.ps1",
];
const CODEX: &[&str] = &["codex-hook.sh", "codex-hook.cmd", "codex-hook.ps1"];
const CURSOR: &[&str] = &["cursor-hook.sh", "cursor-hook.cmd", "cursor-hook.ps1"];
const GEMINI: &[&str] = &["gemini-hook.sh", "gemini-hook.cmd", "gemini-hook.ps1"];
const DROID: &[&str] = &["droid-hook.sh", "droid-hook.cmd", "droid-hook.ps1"];
const COMMAND_CODE: &[&str] = &[
    "command-code-hook.sh",
    "command-code-hook.cmd",
    "command-code-hook.ps1",
];
const ANTIGRAVITY: &[&str] = &[
    "antigravity-hook.sh",
    "antigravity-hook.cmd",
    "antigravity-hook.ps1",
    "antigravity-pre-invocation.cmd",
    "antigravity-post-invocation.cmd",
    "antigravity-stop.cmd",
    "antigravity-pre-tool-use.cmd",
    "antigravity-post-tool-use.cmd",
];
const COPILOT: &[&str] = &["copilot-hook.sh", "copilot-hook.cmd", "copilot-hook.ps1"];
const GROK: &[&str] = &["grok-hook.sh", "grok-hook.cmd", "grok-hook.ps1"];
const DEVIN: &[&str] = &["devin-hook.sh", "devin-hook.cmd", "devin-hook.ps1"];

pub(crate) fn inspect(context: &ScanContext, report: &mut ScanReport) {
    for scope in context.home_scopes() {
        for spec in SPECS {
            inspect_spec(&scope, &scope.home.join(spec.relative_path), spec, report);
        }
        inspect_spec(
            &scope,
            &scope.home.join(".copilot/hooks/orca.json"),
            &ConfigSpec {
                relative_path: "",
                product: "GitHub Copilot",
                markers: COPILOT,
                containers: &["hooks"],
                status_line: false,
                format: FileFormat::Json,
                owned_file: true,
            },
            report,
        );
        inspect_spec(
            &scope,
            &grok_config(context, &scope),
            &ConfigSpec {
                relative_path: "",
                product: "Grok",
                markers: GROK,
                containers: &["hooks"],
                status_line: false,
                format: FileFormat::Json,
                owned_file: true,
            },
            report,
        );
        inspect_spec(
            &scope,
            &devin_config(context, &scope),
            &ConfigSpec {
                relative_path: "",
                product: "Devin",
                markers: DEVIN,
                containers: &["hooks"],
                status_line: false,
                format: FileFormat::Jsonc,
                owned_file: false,
            },
            report,
        );
    }
}

fn grok_config(context: &ScanContext, scope: &HomeScope) -> PathBuf {
    if scope.kind == crate::model::ScopeKind::Local && context.honor_environment {
        if let Some(home) = context.environment_path("GROK_HOME") {
            return home.join("hooks/orca-status.json");
        }
    }
    scope.home.join(".grok/hooks/orca-status.json")
}

fn devin_config(context: &ScanContext, scope: &HomeScope) -> PathBuf {
    if context.platform == Platform::Windows {
        if scope.kind == crate::model::ScopeKind::Local && context.honor_environment {
            if let Some(app_data) = context.environment_path("APPDATA") {
                return app_data.join("devin/config.json");
            }
        }
        return scope.home.join("AppData/Roaming/devin/config.json");
    }
    scope.home.join(".config/devin/config.json")
}

fn inspect_spec(
    scope: &HomeScope,
    configured_path: &Path,
    spec: &ConfigSpec,
    report: &mut ScanReport,
) {
    let Some((path, bytes, root, is_symlink)) = read_config(configured_path, spec, report) else {
        return;
    };
    let Some(object) = root.value().and_then(|node| node.as_object()) else {
        return;
    };
    let mut mutations = 0;
    for container in spec.containers {
        if let Some(property) = object.get(container) {
            if let Some(events) = property.value().and_then(|value| value.as_object()) {
                mutations += clean_events(&events, spec.markers);
                if events.properties().is_empty() && *container != "hooks" {
                    property.remove();
                }
            }
        }
    }
    if spec.status_line {
        if let Some(property) = object.get("statusLine") {
            if property
                .value()
                .is_some_and(|value| node_has_managed_command(&value, spec.markers))
            {
                property.remove();
                mutations += 1;
            }
        }
    }
    if mutations == 0 {
        return;
    }
    let semantically_empty = object.to_serde_value().is_some_and(|value| {
        value.as_object().is_some_and(|map| {
            map.is_empty()
                || (map.len() == 1
                    && map
                        .get("hooks")
                        .and_then(serde_json::Value::as_object)
                        .is_some_and(serde_json::Map::is_empty))
        })
    });
    if spec.owned_file && !is_symlink && semantically_empty {
        add_path(
            report,
            configured_path,
            FindingSpec {
                kind: ArtifactKind::ConfigMutation,
                ownership: Ownership::ProviderOwned,
                safety: Safety::Automatic,
                description: "Orca-owned integration hook configuration",
                evidence: "the Orca-named file contains only exact Orca-managed hook entries",
                id_prefix: "remove-owned-hook-config",
                scope: scope.kind,
                actionable: true,
            },
        );
        return;
    }
    add_rewrite(
        report,
        path,
        &bytes,
        root.to_string().into_bytes(),
        RewriteSpec {
            kind: ArtifactKind::ConfigMutation,
            safety: Safety::Automatic,
            description: format!(
                "Remove Orca-managed entries from {} configuration",
                spec.product
            ),
            evidence: format!(
                "{mutations} entry/entries invoke an exact Orca-managed script filename"
            ),
            id_prefix: "rewrite-agent-config",
            scope: scope.kind,
            format: spec.format,
            mutation_count: mutations,
        },
    );
}

fn read_config(
    configured_path: &Path,
    spec: &ConfigSpec,
    report: &mut ScanReport,
) -> Option<(PathBuf, Vec<u8>, CstRootNode, bool)> {
    let is_symlink = fs::symlink_metadata(configured_path)
        .is_ok_and(|metadata| metadata.file_type().is_symlink());
    let path = resolve_file(configured_path, report)?;
    let bytes = fs::read(&path)
        .map_err(|error| {
            report
                .warnings
                .push(format!("could not read {}: {error}", path.display()));
        })
        .ok()?;
    let text = std::str::from_utf8(&bytes)
        .map_err(|error| {
            report
                .warnings
                .push(format!("{} is not UTF-8: {error}", path.display()));
        })
        .ok()?;
    let root = CstRootNode::parse(text, &ParseOptions::default())
        .map_err(|error| {
            report.warnings.push(format!(
                "could not parse {} configuration {}: {error}",
                spec.product,
                path.display()
            ));
        })
        .ok()?;
    Some((path, bytes, root, is_symlink))
}

fn clean_events(events: &CstObject, markers: &[&str]) -> usize {
    let mut mutations = 0;
    for event in events.properties() {
        let Some(array) = event.value().and_then(|value| value.as_array()) else {
            continue;
        };
        for definition in array.elements() {
            if node_has_direct_command(&definition, markers) {
                definition.remove();
                mutations += 1;
                continue;
            }
            let Some(object) = definition.as_object() else {
                continue;
            };
            let Some(hooks_property) = object.get("hooks") else {
                continue;
            };
            let Some(hooks) = hooks_property.value().and_then(|value| value.as_array()) else {
                continue;
            };
            let before = hooks.elements().len();
            for hook in hooks.elements() {
                if node_has_managed_command(&hook, markers) {
                    hook.remove();
                    mutations += 1;
                }
            }
            if before > 0 && hooks.elements().is_empty() {
                definition.remove();
            }
        }
        if array.elements().is_empty() {
            event.remove();
        }
    }
    mutations
}

fn node_has_direct_command(node: &CstNode, markers: &[&str]) -> bool {
    let Some(object) = node.as_object() else {
        return false;
    };
    ["command", "bash", "powershell"].iter().any(|key| {
        object
            .get(key)
            .and_then(|property| property.value())
            .and_then(|value| value.as_string_lit())
            .and_then(|value| value.decoded_value().ok())
            .is_some_and(|command| command_has_marker(&command, markers))
    })
}

fn node_has_managed_command(node: &CstNode, markers: &[&str]) -> bool {
    node_has_direct_command(node, markers)
        || node.as_object().is_some_and(|object| {
            object.get("hooks").is_some_and(|property| {
                property
                    .value()
                    .and_then(|value| value.as_array())
                    .is_some_and(|array| {
                        array
                            .elements()
                            .iter()
                            .any(|value| node_has_direct_command(value, markers))
                    })
            })
        })
}

fn command_has_marker(command: &str, markers: &[&str]) -> bool {
    let command = command.to_ascii_lowercase().replace('\\', "/");
    markers.iter().any(|marker| {
        command.match_indices(marker).any(|(start, matched)| {
            let before = command[..start].chars().next_back();
            let after = command[start + matched.len()..].chars().next();
            before.is_none_or(|character| !is_filename_character(character))
                && after.is_none_or(|character| !is_filename_character(character))
        })
    })
}

fn is_filename_character(character: char) -> bool {
    character.is_ascii_alphanumeric() || matches!(character, '.' | '_' | '-')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filename_matching_has_boundaries() {
        assert!(command_has_marker(
            "sh ~/.orca/agent-hooks/claude-hook.sh",
            CLAUDE
        ));
        assert!(!command_has_marker("my-claude-hook.sh", CLAUDE));
    }
}
