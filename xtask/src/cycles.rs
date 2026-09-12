//! Workspace dependency-cycle detection for `cargo xtask check-cycles`.
//!
//! Parses `cargo metadata --format-version 1 --no-deps` and builds the
//! intra-workspace crate dependency graph — only edges whose target is another
//! workspace member and that resolve locally (`source` is absent). The first
//! back-edge found by a depth-first traversal is reported as the offending path
//! (`a -> b -> c -> a`). Graph construction and traversal are pure functions so
//! they can be unit-tested without a fixture workspace.

use std::collections::{BTreeMap, BTreeSet};
use std::process::Command;

use serde_json::Value;

use crate::FAILURE;

/// DFS visit state used by the cycle detector.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Color {
    /// Not yet visited.
    White,
    /// On the current DFS stack (a back-edge to this node is a cycle).
    Gray,
    /// Fully explored.
    Black,
}

/// Run `cargo metadata` and fail when the workspace dependency graph has a cycle.
///
/// Prints the member/edge summary on success and the offending `a -> b -> c -> a`
/// path on failure; returns a process exit code.
pub fn run() -> i32 {
    let output = match Command::new("cargo")
        .args(["metadata", "--format-version", "1", "--no-deps"])
        .output()
    {
        Ok(output) => output,
        Err(error) => {
            eprintln!("xtask: cargo metadata could not run: {error}");
            return FAILURE;
        }
    };
    if !output.status.success() {
        eprintln!(
            "xtask: cargo metadata failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        return FAILURE;
    }
    let metadata: Value = match serde_json::from_slice(&output.stdout) {
        Ok(metadata) => metadata,
        Err(error) => {
            eprintln!("xtask: could not parse cargo metadata JSON: {error}");
            return FAILURE;
        }
    };

    let (members, graph) = workspace_graph(&metadata);
    let edges: usize = graph.values().map(Vec::len).sum();
    match find_cycle(&graph) {
        Some(cycle) => {
            eprintln!(
                "xtask: workspace dependency cycle detected: {}",
                cycle.join(" -> ")
            );
            FAILURE
        }
        None => {
            println!(
                "xtask: workspace DAG acyclic — {} member(s), {} edge(s) checked.",
                members.len(),
                edges
            );
            0
        }
    }
}

/// Build the workspace dependency graph from parsed `cargo metadata` output.
///
/// Returns the workspace member names and an adjacency map of member → direct
/// member dependencies. External (registry/git) dependencies are ignored: an
/// edge exists only when the dependency name is a workspace member and it
/// resolves locally (`source` is null/absent).
fn workspace_graph(metadata: &Value) -> (BTreeSet<String>, BTreeMap<String, Vec<String>>) {
    let members = member_names(metadata);
    let mut graph: BTreeMap<String, Vec<String>> = members
        .iter()
        .map(|name| (name.clone(), Vec::new()))
        .collect();

    for package in metadata["packages"].as_array().into_iter().flatten() {
        let Some(name) = package["name"].as_str() else {
            continue;
        };
        if !members.contains(name) {
            continue;
        }
        let mut deps: Vec<String> = package["dependencies"]
            .as_array()
            .into_iter()
            .flatten()
            .filter(|dependency| dependency["source"].is_null())
            .filter_map(|dependency| dependency["name"].as_str())
            .filter(|dependency| members.contains(*dependency) && *dependency != name)
            .map(str::to_string)
            .collect();
        deps.sort();
        deps.dedup();
        if let Some(edges) = graph.get_mut(name) {
            *edges = deps;
        }
    }

    (members, graph)
}

/// Collect the names of the workspace member packages from `cargo metadata`.
fn member_names(metadata: &Value) -> BTreeSet<String> {
    let ids: BTreeSet<&str> = metadata["workspace_members"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .collect();
    metadata["packages"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|package| package["id"].as_str().is_some_and(|id| ids.contains(id)))
        .filter_map(|package| package["name"].as_str())
        .map(str::to_string)
        .collect()
}

/// Find the first dependency cycle via depth-first search.
///
/// Returns the offending path in traversal order with the repeated node appended
/// (`a -> b -> c -> a`), or `None` when the graph is acyclic.
fn find_cycle(graph: &BTreeMap<String, Vec<String>>) -> Option<Vec<String>> {
    let mut state: BTreeMap<&str, Color> = graph
        .keys()
        .map(|name| (name.as_str(), Color::White))
        .collect();
    let mut path: Vec<&str> = Vec::new();

    for name in graph.keys() {
        if state.get(name.as_str()).copied() == Some(Color::White) {
            if let Some(cycle) = visit(name.as_str(), graph, &mut state, &mut path) {
                return Some(cycle);
            }
        }
    }
    None
}

/// Recursive DFS step: colour `node` gray and recurse into its neighbours.
///
/// A gray neighbour is a back-edge; the cycle is sliced out of the current path.
fn visit<'a>(
    node: &'a str,
    graph: &'a BTreeMap<String, Vec<String>>,
    state: &mut BTreeMap<&'a str, Color>,
    path: &mut Vec<&'a str>,
) -> Option<Vec<String>> {
    state.insert(node, Color::Gray);
    path.push(node);

    if let Some(neighbours) = graph.get(node) {
        for next in neighbours {
            let next = next.as_str();
            match state.get(next).copied() {
                Some(Color::Gray) => {
                    let start = path.iter().position(|entry| *entry == next).unwrap_or(0);
                    let mut cycle: Vec<String> = path[start..]
                        .iter()
                        .map(|entry| (*entry).to_string())
                        .collect();
                    cycle.push(next.to_string());
                    return Some(cycle);
                }
                Some(Color::White) => {
                    if let Some(cycle) = visit(next, graph, state, path) {
                        return Some(cycle);
                    }
                }
                _ => {}
            }
        }
    }

    path.pop();
    state.insert(node, Color::Black);
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// Build an adjacency map from `(from, to)` edge pairs.
    fn graph(edges: &[(&str, &str)]) -> BTreeMap<String, Vec<String>> {
        let mut graph: BTreeMap<String, Vec<String>> = BTreeMap::new();
        for (from, to) in edges {
            graph
                .entry((*from).to_string())
                .or_default()
                .push((*to).to_string());
            graph.entry((*to).to_string()).or_default();
        }
        for neighbours in graph.values_mut() {
            neighbours.sort();
            neighbours.dedup();
        }
        graph
    }

    /// An acyclic DAG yields no cycle.
    #[test]
    fn acyclic_graph_has_no_cycle() {
        let graph = graph(&[("app", "orm"), ("orm", "config"), ("app", "config")]);
        assert_eq!(find_cycle(&graph), None);
    }

    /// An injected back-edge is reported as a full path.
    #[test]
    fn injected_cycle_is_reported() {
        let graph = graph(&[("a", "b"), ("b", "c"), ("c", "a")]);
        assert_eq!(
            find_cycle(&graph),
            Some(vec![
                "a".to_string(),
                "b".to_string(),
                "c".to_string(),
                "a".to_string(),
            ])
        );
    }

    /// A self-edge is reported as a one-node cycle.
    #[test]
    fn self_cycle_is_reported() {
        let graph = graph(&[("a", "a")]);
        assert_eq!(
            find_cycle(&graph),
            Some(vec!["a".to_string(), "a".to_string()])
        );
    }

    /// Graph parsing keeps only local workspace edges.
    #[test]
    fn workspace_graph_ignores_external_dependencies() {
        let metadata = json!({
            "workspace_members": [
                "path+file:///ws#app@0.1.0",
                "path+file:///ws/orm#orm@0.1.0",
            ],
            "packages": [
                {
                    "id": "path+file:///ws#app@0.1.0",
                    "name": "app",
                    "dependencies": [
                        { "name": "orm", "source": null, "path": "/ws/orm" },
                        { "name": "serde", "source": "registry+https://github.com/rust-lang/crates.io-index" },
                    ],
                },
                {
                    "id": "path+file:///ws/orm#orm@0.1.0",
                    "name": "orm",
                    "dependencies": [],
                },
            ],
        });

        let (members, graph) = workspace_graph(&metadata);
        assert_eq!(members.len(), 2);
        assert_eq!(graph.get("app"), Some(&vec!["orm".to_string()]));
        assert!(graph.get("orm").is_some_and(Vec::is_empty));
    }
}
