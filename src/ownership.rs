//! Static ownership, borrowing, and resource-lifetime analysis for parsed BHCP.

use std::collections::{BTreeMap, BTreeSet};

use crate::diagnostic::{Diagnostic, Result};
use crate::model::{AstNode, Point};
use crate::parser::{ParsedProgram, SurfaceClauseKind, SurfaceType};
use crate::value::Value;

const INVALID_OWNERSHIP: &str = "BHCP4401";
const BORROW_CONFLICT: &str = "BHCP4402";
const USE_AFTER_MOVE: &str = "BHCP4403";
const INVALID_LIFETIME: &str = "BHCP4404";
const INCONSISTENT_LINEAR_USE: &str = "BHCP4405";

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct OwnershipReport {
    pub checked_goals: usize,
    pub checked_bindings: usize,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct PersistentShareApproval {
    pub goal: String,
    pub binding: String,
    pub lifetime: String,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct OwnershipPolicy {
    pub persistent_shares: BTreeSet<PersistentShareApproval>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Ownership {
    Owned,
    Shared,
    Borrowed,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Access {
    Read,
    Write,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Usage {
    Unrestricted,
    Affine,
    Linear,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct HandleSpec {
    ownership: Ownership,
    access: Access,
    usage: Usage,
    lifetime: String,
    resource: String,
}

#[derive(Clone, Debug)]
struct BindingSpec {
    handle: HandleSpec,
    at: Point,
}

#[derive(Clone, Debug, Default)]
struct GoalInfo {
    inputs: BTreeMap<String, BindingSpec>,
    bindings: BTreeMap<String, BindingSpec>,
}

#[derive(Clone, Debug)]
enum BindingState {
    Live,
    Moved { branch: String, at: Point },
}

#[derive(Clone, Debug)]
struct PathState {
    bindings: BTreeMap<String, BindingState>,
    path: Vec<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Operation {
    Read,
    WriteBorrow,
    Move,
    Share,
}

#[derive(Clone, Debug)]
struct AccessEvent {
    binding: String,
    resource: String,
    lifetime: String,
    operation: Operation,
    branch: String,
    at: Point,
}

#[derive(Clone, Debug)]
struct AnalysisPath {
    state: PathState,
    events: Vec<AccessEvent>,
}

pub fn analyze_program(program: &ParsedProgram, source_name: &str) -> Result<OwnershipReport> {
    analyze_program_with_policy(program, source_name, &OwnershipPolicy::default())
}

pub fn analyze_program_with_policy(
    program: &ParsedProgram,
    source_name: &str,
    policy: &OwnershipPolicy,
) -> Result<OwnershipReport> {
    let mut goals = BTreeMap::new();
    let mut checked_bindings = 0;
    for goal in &program.goals {
        let info = goal_info(goal, source_name)?;
        checked_bindings += info.bindings.len();
        goals.insert(goal.symbol.clone(), info);
    }

    let analyzer = Analyzer {
        goals: &goals,
        source_name,
        policy,
    };
    for goal in &program.goals {
        analyzer.analyze_goal(goal, &goals[&goal.symbol])?;
    }

    Ok(OwnershipReport {
        checked_goals: program.goals.len(),
        checked_bindings,
    })
}

fn goal_info(goal: &crate::parser::SurfaceGoal, source_name: &str) -> Result<GoalInfo> {
    let mut info = GoalInfo::default();
    for clause in &goal.clauses {
        if let SurfaceClauseKind::Fact {
            kind,
            name,
            value_type,
        } = &clause.kind
            && let Some(handle) = handle_from_surface(value_type, source_name, &clause.at)?
        {
            let binding = BindingSpec {
                handle,
                at: clause.at.clone(),
            };
            info.bindings.insert(name.clone(), binding.clone());
            if *kind == "input" {
                info.inputs.insert(name.clone(), binding);
            }
        }
    }

    // Resource/state clauses and structured unsupported facts retain their exact
    // type in the canonical AST even before their later lowering stages exist.
    for node in &goal.ast.children {
        if !matches!(
            node.kind.as_str(),
            "input" | "output" | "resource" | "state"
        ) {
            continue;
        }
        let Some(name) = text_attribute(node, "name") else {
            continue;
        };
        let Some(value_type) = attribute(node, "type") else {
            continue;
        };
        let Some(handle) = handle_from_value(value_type, source_name, &node.span.start)? else {
            continue;
        };
        let binding = BindingSpec {
            handle,
            at: node.span.start.clone(),
        };
        info.bindings.insert(name.to_owned(), binding.clone());
        if node.kind == "input" {
            info.inputs.insert(name.to_owned(), binding);
        }
    }
    Ok(info)
}

fn handle_from_surface(
    value_type: &SurfaceType,
    source_name: &str,
    at: &Point,
) -> Result<Option<HandleSpec>> {
    let SurfaceType::Handle {
        ownership,
        access,
        usage,
        lifetime,
        value_type,
    } = value_type
    else {
        return Ok(None);
    };
    handle_spec(
        ownership,
        access.as_deref(),
        usage.as_deref(),
        lifetime.as_deref(),
        surface_type_label(value_type),
        source_name,
        at,
    )
    .map(Some)
}

fn handle_from_value(value: &Value, source_name: &str, at: &Point) -> Result<Option<HandleSpec>> {
    let Value::Array(parts) = value else {
        return Ok(None);
    };
    let [
        Value::Text(kind),
        Value::Text(ownership),
        access,
        usage,
        lifetime,
        value_type,
    ] = parts.as_slice()
    else {
        return Ok(None);
    };
    if kind != "handle" {
        return Ok(None);
    }
    handle_spec(
        ownership,
        optional_text(access),
        optional_text(usage),
        optional_text(lifetime),
        value_type_label(value_type),
        source_name,
        at,
    )
    .map(Some)
}

fn handle_spec(
    ownership: &str,
    access: Option<&str>,
    usage: Option<&str>,
    lifetime: Option<&str>,
    resource: String,
    source_name: &str,
    at: &Point,
) -> Result<HandleSpec> {
    let ownership = match ownership {
        "owned" => Ownership::Owned,
        "shared" => Ownership::Shared,
        "borrowed" => Ownership::Borrowed,
        _ => {
            return Err(error(
                INVALID_OWNERSHIP,
                "unknown handle ownership mode",
                source_name,
                at,
            ));
        }
    };
    if ownership == Ownership::Borrowed && access.is_none() {
        return Err(error(
            INVALID_OWNERSHIP,
            "borrowed handle must state read or write access",
            source_name,
            at,
        ));
    }
    let access = match access.unwrap_or(if ownership == Ownership::Shared {
        "read"
    } else {
        "write"
    }) {
        "read" => Access::Read,
        "write" => Access::Write,
        _ => {
            return Err(error(
                INVALID_OWNERSHIP,
                "unknown handle access mode",
                source_name,
                at,
            ));
        }
    };
    if ownership == Ownership::Shared && access == Access::Write {
        return Err(error(
            INVALID_OWNERSHIP,
            "shared handle cannot grant write access",
            source_name,
            at,
        ));
    }
    let usage = match usage.unwrap_or("unrestricted") {
        "unrestricted" => Usage::Unrestricted,
        "affine" => Usage::Affine,
        "linear" => Usage::Linear,
        _ => {
            return Err(error(
                INVALID_OWNERSHIP,
                "unknown handle usage mode",
                source_name,
                at,
            ));
        }
    };
    Ok(HandleSpec {
        ownership,
        access,
        usage,
        lifetime: lifetime.unwrap_or("goal").to_owned(),
        resource,
    })
}

struct Analyzer<'a> {
    goals: &'a BTreeMap<String, GoalInfo>,
    source_name: &'a str,
    policy: &'a OwnershipPolicy,
}

impl Analyzer<'_> {
    fn analyze_goal(&self, goal: &crate::parser::SurfaceGoal, info: &GoalInfo) -> Result<()> {
        self.check_persistent_state(goal, info)?;

        let initial = PathState {
            bindings: info
                .bindings
                .keys()
                .cloned()
                .map(|name| (name, BindingState::Live))
                .collect(),
            path: Vec::new(),
        };
        let mut paths = vec![AnalysisPath {
            state: initial,
            events: Vec::new(),
        }];
        let body_nodes: Vec<_> = goal
            .ast
            .children
            .iter()
            .filter(|node| is_flow_node(node))
            .collect();
        for node in body_nodes {
            paths = self.sequence(node, paths, info)?;
        }

        if !goal.ast.children.iter().any(is_flow_node) {
            return Ok(());
        }
        for (name, binding) in &info.bindings {
            if binding.handle.usage != Usage::Linear {
                continue;
            }
            let moved: Vec<_> = paths
                .iter()
                .filter(|path| {
                    matches!(
                        path.state.bindings.get(name),
                        Some(BindingState::Moved { .. })
                    )
                })
                .map(path_name)
                .collect();
            let live: Vec<_> = paths
                .iter()
                .filter(|path| matches!(path.state.bindings.get(name), Some(BindingState::Live)))
                .map(path_name)
                .collect();
            if !moved.is_empty() && !live.is_empty() {
                return Err(error(
                    INCONSISTENT_LINEAR_USE,
                    format!(
                        "linear resource {:?} binding {name:?} with lifetime {:?} is consumed on branches [{}] but remains live on branches [{}]",
                        binding.handle.resource,
                        binding.handle.lifetime,
                        moved.join(", "),
                        live.join(", ")
                    ),
                    self.source_name,
                    &binding.at,
                ));
            }
            if moved.is_empty() {
                return Err(error(
                    INCONSISTENT_LINEAR_USE,
                    format!(
                        "linear resource {:?} binding {name:?} with lifetime {:?} is not consumed by the goal body on every outcome",
                        binding.handle.resource, binding.handle.lifetime
                    ),
                    self.source_name,
                    &binding.at,
                ));
            }
        }
        Ok(())
    }

    fn check_persistent_state(
        &self,
        goal: &crate::parser::SurfaceGoal,
        info: &GoalInfo,
    ) -> Result<()> {
        for node in &goal.ast.children {
            if node.kind != "state" {
                continue;
            }
            let Some(state_name) = text_attribute(node, "name") else {
                continue;
            };
            let Some(initializer) = attribute(node, "initializer") else {
                continue;
            };
            let mut sources = BTreeSet::new();
            collect_reference_names(initializer, &mut sources);
            for source in sources {
                let Some(binding) = info.bindings.get(source.as_str()) else {
                    continue;
                };
                if binding.handle.ownership == Ownership::Borrowed {
                    return Err(error(
                        INVALID_LIFETIME,
                        format!(
                            "persistent state {state_name:?} cannot retain borrowed resource {:?} from binding {source:?} with expiring lifetime {:?}",
                            binding.handle.resource, binding.handle.lifetime
                        ),
                        self.source_name,
                        &node.span.start,
                    ));
                }
                if binding.handle.ownership == Ownership::Shared
                    && !self
                        .policy
                        .persistent_shares
                        .contains(&PersistentShareApproval {
                            goal: goal.symbol.clone(),
                            binding: source.clone(),
                            lifetime: binding.handle.lifetime.clone(),
                        })
                {
                    return Err(error(
                        INVALID_LIFETIME,
                        format!(
                            "persistent state {state_name:?} requires an owned value or policy-approved persistent share; resource {:?} binding {source:?} has lifetime {:?}",
                            binding.handle.resource, binding.handle.lifetime
                        ),
                        self.source_name,
                        &node.span.start,
                    ));
                }
            }
        }
        Ok(())
    }

    fn sequence(
        &self,
        node: &AstNode,
        paths: Vec<AnalysisPath>,
        scope: &GoalInfo,
    ) -> Result<Vec<AnalysisPath>> {
        let mut next = Vec::new();
        for path in paths {
            next.extend(self.analyze_node(node, path, scope)?);
        }
        Ok(deduplicate_paths(next))
    }

    fn analyze_node(
        &self,
        node: &AstNode,
        path: AnalysisPath,
        scope: &GoalInfo,
    ) -> Result<Vec<AnalysisPath>> {
        match node.kind.as_str() {
            "branch" => self.analyze_branch(node, path, scope),
            "chain" => {
                let mut paths = vec![path];
                for child in &node.children {
                    paths = self.sequence(child, paths, scope)?;
                }
                Ok(paths)
            }
            "gate" => {
                let mut outcomes = vec![path.clone()];
                for child in &node.children {
                    outcomes.extend(self.analyze_node(child, path.clone(), scope)?);
                }
                Ok(deduplicate_paths(outcomes))
            }
            "all" => self.analyze_all(node, path, scope),
            "any" | "none" | "compose" => self.analyze_candidates(node, path, scope),
            "goal-call" => self.analyze_call(node, path, scope, "goal-call"),
            _ => Ok(vec![path]),
        }
    }

    fn analyze_branch(
        &self,
        node: &AstNode,
        mut path: AnalysisPath,
        scope: &GoalInfo,
    ) -> Result<Vec<AnalysisPath>> {
        let branch = text_attribute(node, "tag").unwrap_or("branch");
        path.state.path.push(branch.to_owned());
        if text_attribute(node, "goal").is_some() {
            self.analyze_call(node, path, scope, branch)
        } else {
            let mut paths = vec![path];
            for child in &node.children {
                paths = self.sequence(child, paths, scope)?;
            }
            Ok(paths)
        }
    }

    fn analyze_call(
        &self,
        node: &AstNode,
        mut path: AnalysisPath,
        scope: &GoalInfo,
        branch: &str,
    ) -> Result<Vec<AnalysisPath>> {
        let target = text_attribute(node, "goal").and_then(|goal| self.goals.get(goal));
        let mut call_events = Vec::new();
        for argument in &node.children {
            if argument.kind != "argument" {
                continue;
            }
            let Some(parameter) = text_attribute(argument, "name") else {
                continue;
            };
            let Some(source) = text_attribute(argument, "source")
                .or_else(|| attribute(argument, "value").and_then(reference_name))
            else {
                continue;
            };
            let Some(source_binding) = scope.bindings.get(source) else {
                continue;
            };
            let target_binding = target.and_then(|goal| goal.inputs.get(parameter));
            let mode = text_attribute(argument, "mode").unwrap_or("value");
            let operation = self.validate_argument_mode(
                mode,
                source,
                source_binding,
                target_binding,
                &argument.span.start,
            )?;
            self.require_live(
                &path.state,
                source,
                branch,
                &argument.span.start,
                source_binding,
            )?;
            if operation == Operation::Move {
                path.state.bindings.insert(
                    source.to_owned(),
                    BindingState::Moved {
                        branch: branch.to_owned(),
                        at: argument.span.start.clone(),
                    },
                );
            }
            call_events.push(AccessEvent {
                binding: source.to_owned(),
                resource: source_binding.handle.resource.clone(),
                lifetime: source_binding.handle.lifetime.clone(),
                operation,
                branch: branch.to_owned(),
                at: argument.span.start.clone(),
            });
        }
        self.check_conflicts(&call_events)?;
        path.events.extend(call_events);
        Ok(vec![path])
    }

    fn validate_argument_mode(
        &self,
        mode: &str,
        source: &str,
        source_binding: &BindingSpec,
        target: Option<&BindingSpec>,
        at: &Point,
    ) -> Result<Operation> {
        let source_handle = &source_binding.handle;
        let target_handle = target.map(|binding| &binding.handle);
        match mode {
            "move" => {
                if source_handle.ownership != Ownership::Owned
                    || target_handle.is_some_and(|target| target.ownership != Ownership::Owned)
                {
                    return Err(error(
                        INVALID_OWNERSHIP,
                        format!(
                            "move of resource {:?} binding {source:?} requires owned source and target handles",
                            source_handle.resource
                        ),
                        self.source_name,
                        at,
                    ));
                }
                Ok(Operation::Move)
            }
            "borrow" => {
                let Some(target) = target_handle else {
                    return Ok(Operation::Read);
                };
                if target.ownership != Ownership::Borrowed {
                    return Err(error(
                        INVALID_OWNERSHIP,
                        format!(
                            "borrow of resource {:?} binding {source:?} requires a borrowed target handle",
                            source_handle.resource
                        ),
                        self.source_name,
                        at,
                    ));
                }
                if target.access == Access::Write && source_handle.access != Access::Write {
                    return Err(error(
                        INVALID_OWNERSHIP,
                        format!(
                            "resource {:?} binding {source:?} cannot grant a write borrow from read access",
                            source_handle.resource
                        ),
                        self.source_name,
                        at,
                    ));
                }
                if source_handle.lifetime != target.lifetime {
                    return Err(error(
                        INVALID_LIFETIME,
                        format!(
                            "borrowed resource {:?} binding {source:?} lifetime {:?} cannot cross into target lifetime {:?}",
                            source_handle.resource, source_handle.lifetime, target.lifetime
                        ),
                        self.source_name,
                        at,
                    ));
                }
                Ok(if target.access == Access::Write {
                    Operation::WriteBorrow
                } else {
                    Operation::Read
                })
            }
            "share" => {
                if target_handle.is_some_and(|target| target.ownership != Ownership::Shared)
                    || source_handle.ownership == Ownership::Borrowed
                {
                    return Err(error(
                        INVALID_OWNERSHIP,
                        format!(
                            "share of resource {:?} binding {source:?} requires an owned/shared source and shared target",
                            source_handle.resource
                        ),
                        self.source_name,
                        at,
                    ));
                }
                if let Some(target) = target_handle
                    && source_handle.lifetime != target.lifetime
                {
                    return Err(error(
                        INVALID_LIFETIME,
                        format!(
                            "shared resource {:?} binding {source:?} lifetime {:?} cannot cross into target lifetime {:?}",
                            source_handle.resource, source_handle.lifetime, target.lifetime
                        ),
                        self.source_name,
                        at,
                    ));
                }
                Ok(Operation::Share)
            }
            "value" => {
                if source_handle.ownership == Ownership::Owned {
                    return Err(error(
                        INVALID_OWNERSHIP,
                        format!(
                            "owned resource {:?} binding {source:?} must be passed explicitly by move, borrow, or share",
                            source_handle.resource
                        ),
                        self.source_name,
                        at,
                    ));
                }
                Ok(Operation::Read)
            }
            _ => Err(error(
                INVALID_OWNERSHIP,
                format!("unknown argument ownership mode {mode:?}"),
                self.source_name,
                at,
            )),
        }
    }

    fn require_live(
        &self,
        state: &PathState,
        binding: &str,
        current_branch: &str,
        at: &Point,
        spec: &BindingSpec,
    ) -> Result<()> {
        let Some(BindingState::Moved {
            branch: moved_branch,
            at: moved_at,
        }) = state.bindings.get(binding)
        else {
            return Ok(());
        };
        Err(error(
            USE_AFTER_MOVE,
            format!(
                "resource {:?} binding {binding:?} with lifetime {:?} was moved by branch {moved_branch:?} at {}:{}:{} and reused by branch {current_branch:?}",
                spec.handle.resource,
                spec.handle.lifetime,
                self.source_name,
                moved_at.line,
                moved_at.column
            ),
            self.source_name,
            at,
        ))
    }

    fn analyze_candidates(
        &self,
        node: &AstNode,
        path: AnalysisPath,
        scope: &GoalInfo,
    ) -> Result<Vec<AnalysisPath>> {
        let mut outcomes = Vec::new();
        let mut branch_events = Vec::new();
        for child in &node.children {
            let mut events = Vec::new();
            let child_paths = self.analyze_node(child, path.clone(), scope)?;
            for child_path in child_paths {
                events.extend(child_path.events.iter().skip(path.events.len()).cloned());
                outcomes.push(child_path);
            }
            branch_events.push(events);
        }
        self.check_cross_branch_conflicts(&branch_events)?;
        if outcomes.is_empty() {
            outcomes.push(path);
        }
        Ok(deduplicate_paths(outcomes))
    }

    fn analyze_all(
        &self,
        node: &AstNode,
        path: AnalysisPath,
        scope: &GoalInfo,
    ) -> Result<Vec<AnalysisPath>> {
        let mut branch_paths = Vec::new();
        let mut branch_events = Vec::new();
        for child in &node.children {
            let mut events = Vec::new();
            for child_path in self.analyze_node(child, path.clone(), scope)? {
                events.extend(child_path.events.iter().skip(path.events.len()).cloned());
                branch_paths.push(child_path);
            }
            branch_events.push(events);
        }
        self.check_cross_branch_conflicts(&branch_events)?;
        let mut merged = path;
        for branch_path in branch_paths {
            merged.state.path.extend(branch_path.state.path);
            for (name, state) in branch_path.state.bindings {
                if matches!(state, BindingState::Moved { .. }) {
                    merged.state.bindings.insert(name, state);
                }
            }
        }
        merged.events.extend(branch_events.into_iter().flatten());
        Ok(vec![merged])
    }

    fn check_cross_branch_conflicts(&self, branches: &[Vec<AccessEvent>]) -> Result<()> {
        for (index, left_branch) in branches.iter().enumerate() {
            for right_branch in &branches[index + 1..] {
                for left in left_branch {
                    for right in right_branch {
                        self.check_conflict(left, right)?;
                    }
                }
            }
        }
        Ok(())
    }

    fn check_conflicts(&self, events: &[AccessEvent]) -> Result<()> {
        for (index, left) in events.iter().enumerate() {
            for right in &events[index + 1..] {
                self.check_conflict(left, right)?;
            }
        }
        Ok(())
    }

    fn check_conflict(&self, left: &AccessEvent, right: &AccessEvent) -> Result<()> {
        if left.binding != right.binding || compatible(left.operation, right.operation) {
            return Ok(());
        }
        Err(error(
            BORROW_CONFLICT,
            format!(
                "resource {:?} binding {:?} with lifetime {:?} has conflicting accesses in branches {:?} at {}:{}:{} and {:?} at {}:{}:{}",
                left.resource,
                left.binding,
                left.lifetime,
                left.branch,
                self.source_name,
                left.at.line,
                left.at.column,
                right.branch,
                self.source_name,
                right.at.line,
                right.at.column
            ),
            self.source_name,
            &right.at,
        ))
    }
}

fn compatible(left: Operation, right: Operation) -> bool {
    matches!(
        (left, right),
        (
            Operation::Read | Operation::Share,
            Operation::Read | Operation::Share
        )
    )
}

fn is_flow_node(node: &AstNode) -> bool {
    matches!(
        node.kind.as_str(),
        "all" | "any" | "none" | "chain" | "gate" | "compose" | "goal-call"
    )
}

fn attribute<'a>(node: &'a AstNode, name: &str) -> Option<&'a Value> {
    node.attributes
        .iter()
        .find_map(|(candidate, value)| (candidate == name).then_some(value))
}

fn text_attribute<'a>(node: &'a AstNode, name: &str) -> Option<&'a str> {
    match attribute(node, name) {
        Some(Value::Text(value)) => Some(value),
        _ => None,
    }
}

fn optional_text(value: &Value) -> Option<&str> {
    match value {
        Value::Text(value) => Some(value),
        Value::Null => None,
        _ => None,
    }
}

fn surface_type_label(value_type: &SurfaceType) -> String {
    match value_type {
        SurfaceType::Primitive(name) | SurfaceType::Exact(name) => (*name).to_owned(),
        SurfaceType::Nominal { symbol, .. } => symbol.clone(),
        SurfaceType::Record(_) | SurfaceType::StructuralRecord { .. } => "record".to_owned(),
        SurfaceType::Variant(_) => "variant".to_owned(),
        SurfaceType::Tuple(_) => "tuple".to_owned(),
        SurfaceType::List(_) => "list".to_owned(),
        SurfaceType::Set(_) => "set".to_owned(),
        SurfaceType::Map { .. } => "map".to_owned(),
        SurfaceType::Option(_) => "option".to_owned(),
        SurfaceType::Result { .. } => "result".to_owned(),
        SurfaceType::Goal { .. } => "goal".to_owned(),
        SurfaceType::Union(_) => "union".to_owned(),
        SurfaceType::Intersection(_) => "intersection".to_owned(),
        SurfaceType::Handle { value_type, .. } | SurfaceType::Refined { value_type, .. } => {
            surface_type_label(value_type)
        }
        SurfaceType::Parameter(name) => name.clone(),
        SurfaceType::Dynamic => "Dynamic".to_owned(),
        SurfaceType::Reduction(_) => "reduction".to_owned(),
        SurfaceType::Meta { .. } => "meta".to_owned(),
        SurfaceType::Never => "Never".to_owned(),
    }
}

fn value_type_label(value_type: &Value) -> String {
    match value_type {
        Value::Array(parts) => match parts.as_slice() {
            [Value::Text(kind), Value::Text(name), ..]
                if matches!(kind.as_str(), "primitive" | "exact-number" | "nominal") =>
            {
                name.clone()
            }
            [Value::Text(kind), ..] => kind.clone(),
            _ => "resource".to_owned(),
        },
        _ => "resource".to_owned(),
    }
}

fn reference_name(value: &Value) -> Option<&str> {
    match value {
        Value::Array(parts) if matches!(parts.as_slice(), [Value::Text(kind), Value::Text(_)] if kind == "reference") => {
            match &parts[1] {
                Value::Text(name) => Some(name),
                _ => None,
            }
        }
        _ => None,
    }
}

fn collect_reference_names(value: &Value, references: &mut BTreeSet<String>) {
    if let Some(name) = reference_name(value) {
        references.insert(name.to_owned());
        return;
    }
    match value {
        Value::Array(values) => {
            for value in values {
                collect_reference_names(value, references);
            }
        }
        Value::Map(entries) => {
            for (_, value) in entries {
                collect_reference_names(value, references);
            }
        }
        Value::Tag(_, value) => collect_reference_names(value, references),
        Value::Null | Value::Bool(_) | Value::Integer(_) | Value::Text(_) | Value::Bytes(_) => {}
    }
}

fn path_name(path: &AnalysisPath) -> String {
    if path.state.path.is_empty() {
        "<direct>".to_owned()
    } else {
        path.state.path.join("/")
    }
}

fn deduplicate_paths(paths: Vec<AnalysisPath>) -> Vec<AnalysisPath> {
    let mut seen = BTreeSet::new();
    paths
        .into_iter()
        .filter(|path| {
            let key = format!("{:?}|{}", path.state.bindings, path_name(path));
            seen.insert(key)
        })
        .collect()
}

fn error(code: &'static str, message: impl Into<String>, source: &str, at: &Point) -> Diagnostic {
    Diagnostic::new(code, message, source, at.line, at.column)
}
