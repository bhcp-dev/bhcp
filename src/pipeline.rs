use std::collections::{BTreeMap, HashMap, HashSet};

use crate::cbor::encode_deterministic;
use crate::definition::DefinitionElaborator;
use crate::diagnostic::{Diagnostic, Result};
use crate::hash::{HashAlgorithm, artifact_hash_with, semantic_hash_with};
use crate::kernel::{ArgumentMode, KernelArgument, KernelChild, KernelNetwork};
use crate::model::{
    BhcpType, Binding, CanonicalAstDocument, Clause, ClauseKind, ContentReference, Effect,
    EffectivePolicyReference, Expression, ExpressionForm, FieldType, FunctionDefinition,
    GoalDefinition, HandleType, HashId, PolicyDecision, SemanticIrDocument, VariantCaseType,
    VerifierBinding, data_edge_type_compatible, features_for, is_symbol,
};
use crate::ownership::analyze_program;
use crate::parser::{
    CANONICAL_PROFILE, ParsedProgram, SurfaceArgumentMode, SurfaceClauseKind, SurfaceComposition,
    SurfaceEffect, SurfaceExpression, SurfaceFunction, SurfaceGoal, SurfaceLiteral, SurfaceType,
    parse_canonical, parse_with_syntax, scan_profile_preamble, validate_effective_syntax,
};
use crate::policy::{
    EffectivePolicyDocument, ExactNumber, PolicyDocument, PolicyScope, SourcePolicyDocument,
    TypeMode,
};
use crate::prelude::{
    ALL_FEATURE, ALL_LOWERER, ALL_REDUCER, ANY_FEATURE, ANY_LOWERER, ANY_REDUCER, CHAIN_FEATURE,
    CHAIN_LOWERER, CHAIN_REDUCER, DerivedChild, DerivedForm, GATE_FEATURE, GATE_LOWERER,
    NONE_FEATURE, NONE_LOWERER, NONE_REDUCER, NetworkShape, Prelude,
};
use crate::profile::{ProfileRegistry, SyntaxDocument};
use crate::typecheck::{CheckedType, check_type_definitions};
use crate::value::Value;

const OWNERSHIP_FEATURE: &str = "bhcp/feature.ownership-analysis@0";

#[derive(Clone, Debug)]
pub struct Compilation {
    pub ast: CanonicalAstDocument,
    pub ir: SemanticIrDocument,
    pub ast_bytes: Vec<u8>,
    pub ir_bytes: Vec<u8>,
    pub ast_hash: HashId,
    pub semantic_hash: HashId,
    pub ir_hash: HashId,
    pub effective_policy: Option<EffectivePolicyDocument>,
}

#[derive(Clone, Debug)]
pub struct ParsedPolicySource {
    pub ast: CanonicalAstDocument,
    pub documents: Vec<SourcePolicyDocument>,
}

#[derive(Clone, Debug, Default)]
pub struct ProfileSyntaxRegistry {
    profiles: BTreeMap<String, SyntaxDocument>,
}

impl ProfileSyntaxRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&mut self, profile: &str, syntax: SyntaxDocument) -> Result<()> {
        if !is_symbol(profile) || profile == CANONICAL_PROFILE {
            return Err(Diagnostic::new(
                "BHCP9002",
                "invalid-profile-registration",
                "<profile-registry>",
                1,
                1,
            ));
        }
        validate_effective_syntax(&syntax)
            .map_err(|diagnostic| attach_profile_context(diagnostic, profile))?;
        if self.profiles.contains_key(profile) {
            return Err(Diagnostic::new(
                "BHCP9002",
                "duplicate-profile-registration",
                "<profile-registry>",
                1,
                1,
            ));
        }
        self.profiles.insert(profile.to_owned(), syntax);
        Ok(())
    }

    fn get(&self, profile: &str) -> Option<&SyntaxDocument> {
        self.profiles.get(profile)
    }
}

pub fn parse_source(source: &str, source_name: &str) -> Result<CanonicalAstDocument> {
    parse_source_with_algorithm(source, source_name, HashAlgorithm::default())
}

pub fn parse_source_with_algorithm(
    source: &str,
    source_name: &str,
    algorithm: HashAlgorithm,
) -> Result<CanonicalAstDocument> {
    parse_source_bytes_with_algorithm(source.as_bytes(), source_name, algorithm)
}

pub fn parse_source_bytes(source: &[u8], source_name: &str) -> Result<CanonicalAstDocument> {
    parse_source_bytes_with_algorithm(source, source_name, HashAlgorithm::default())
}

pub fn parse_source_bytes_with_algorithm(
    source: &[u8],
    source_name: &str,
    algorithm: HashAlgorithm,
) -> Result<CanonicalAstDocument> {
    Ok(parse_internal(source, source_name, algorithm, None)?.0)
}

pub fn parse_source_bytes_with_profiles(
    source: &[u8],
    source_name: &str,
    profiles: &ProfileSyntaxRegistry,
) -> Result<CanonicalAstDocument> {
    parse_source_bytes_with_profiles_and_algorithm(
        source,
        source_name,
        profiles,
        HashAlgorithm::default(),
    )
}

pub fn parse_source_with_profiles(
    source: &str,
    source_name: &str,
    profiles: &ProfileSyntaxRegistry,
) -> Result<CanonicalAstDocument> {
    parse_source_bytes_with_profiles(source.as_bytes(), source_name, profiles)
}

pub fn parse_source_bytes_with_profiles_and_algorithm(
    source: &[u8],
    source_name: &str,
    profiles: &ProfileSyntaxRegistry,
    algorithm: HashAlgorithm,
) -> Result<CanonicalAstDocument> {
    Ok(parse_internal(source, source_name, algorithm, Some(profiles))?.0)
}

pub fn parse_source_bytes_with_profile_registry(
    source: &[u8],
    source_name: &str,
    registry: &ProfileRegistry,
) -> Result<CanonicalAstDocument> {
    parse_source_bytes_with_profile_registry_and_algorithm(
        source,
        source_name,
        registry,
        HashAlgorithm::default(),
    )
}

pub fn parse_source_bytes_with_profile_registry_and_algorithm(
    source: &[u8],
    source_name: &str,
    registry: &ProfileRegistry,
    algorithm: HashAlgorithm,
) -> Result<CanonicalAstDocument> {
    let selected = scan_profile_preamble(source, source_name)?;
    if selected.profile == CANONICAL_PROFILE {
        return Ok(parse_internal(source, source_name, algorithm, None)?.0);
    }
    let resolved = registry.resolve(&selected.profile, algorithm)?;
    let mut syntaxes = ProfileSyntaxRegistry::new();
    syntaxes.register(&selected.profile, resolved.syntax)?;
    Ok(parse_internal(source, source_name, algorithm, Some(&syntaxes))?.0)
}

pub fn parse_policy_source(source: &str, source_name: &str) -> Result<ParsedPolicySource> {
    parse_policy_source_with_algorithm(source, source_name, HashAlgorithm::default())
}

pub fn parse_policy_source_with_algorithm(
    source: &str,
    source_name: &str,
    algorithm: HashAlgorithm,
) -> Result<ParsedPolicySource> {
    parse_policy_source_bytes_with_algorithm(source.as_bytes(), source_name, algorithm)
}

pub fn parse_policy_source_bytes(source: &[u8], source_name: &str) -> Result<ParsedPolicySource> {
    parse_policy_source_bytes_with_algorithm(source, source_name, HashAlgorithm::default())
}

pub fn parse_policy_source_bytes_with_algorithm(
    source: &[u8],
    source_name: &str,
    algorithm: HashAlgorithm,
) -> Result<ParsedPolicySource> {
    let (ast, program) = parse_internal(source, source_name, algorithm, None)?;
    if program.policies.is_empty() {
        return Err(Diagnostic::new(
            "BHCP1001",
            "policy source must contain at least one §policy definition",
            source_name,
            1,
            1,
        ));
    }
    Ok(ParsedPolicySource {
        ast,
        documents: program
            .policies
            .into_iter()
            .map(|policy| policy.document)
            .collect(),
    })
}

pub fn compile_source(source: &str, source_name: &str) -> Result<Compilation> {
    compile_source_with_algorithm(source, source_name, HashAlgorithm::default())
}

pub fn compile_source_with_policy(
    source: &str,
    source_name: &str,
    policy: &EffectivePolicyDocument,
) -> Result<Compilation> {
    let algorithm = policy
        .header
        .semantic_id
        .as_ref()
        .map(|identity| HashAlgorithm::from_id(&identity.algorithm))
        .transpose()?
        .unwrap_or_default();
    compile_source_with_policy_and_algorithm(source, source_name, policy, algorithm)
}

pub fn compile_source_with_policy_and_algorithm(
    source: &str,
    source_name: &str,
    policy: &EffectivePolicyDocument,
    algorithm: HashAlgorithm,
) -> Result<Compilation> {
    compile_source_internal(
        source.as_bytes(),
        source_name,
        algorithm,
        Some(policy),
        None,
    )
}

pub fn compile_source_with_algorithm(
    source: &str,
    source_name: &str,
    algorithm: HashAlgorithm,
) -> Result<Compilation> {
    compile_source_bytes_with_algorithm(source.as_bytes(), source_name, algorithm)
}

pub fn compile_source_bytes(source: &[u8], source_name: &str) -> Result<Compilation> {
    compile_source_bytes_with_algorithm(source, source_name, HashAlgorithm::default())
}

pub fn compile_source_bytes_with_algorithm(
    source: &[u8],
    source_name: &str,
    algorithm: HashAlgorithm,
) -> Result<Compilation> {
    compile_source_internal(source, source_name, algorithm, None, None)
}

pub fn compile_source_bytes_with_profiles(
    source: &[u8],
    source_name: &str,
    profiles: &ProfileSyntaxRegistry,
) -> Result<Compilation> {
    compile_source_bytes_with_profiles_and_algorithm(
        source,
        source_name,
        profiles,
        HashAlgorithm::default(),
    )
}

pub fn compile_source_with_profiles(
    source: &str,
    source_name: &str,
    profiles: &ProfileSyntaxRegistry,
) -> Result<Compilation> {
    compile_source_bytes_with_profiles(source.as_bytes(), source_name, profiles)
}

pub fn compile_source_bytes_with_profiles_and_algorithm(
    source: &[u8],
    source_name: &str,
    profiles: &ProfileSyntaxRegistry,
    algorithm: HashAlgorithm,
) -> Result<Compilation> {
    compile_source_internal(source, source_name, algorithm, None, Some(profiles))
}

pub fn compile_source_bytes_with_profile_registry(
    source: &[u8],
    source_name: &str,
    registry: &ProfileRegistry,
) -> Result<Compilation> {
    compile_source_bytes_with_profile_registry_and_algorithm(
        source,
        source_name,
        registry,
        HashAlgorithm::default(),
    )
}

pub fn compile_source_bytes_with_profile_registry_and_algorithm(
    source: &[u8],
    source_name: &str,
    registry: &ProfileRegistry,
    algorithm: HashAlgorithm,
) -> Result<Compilation> {
    let selected = scan_profile_preamble(source, source_name)?;
    if selected.profile == CANONICAL_PROFILE {
        return compile_source_internal(source, source_name, algorithm, None, None);
    }
    let resolved = registry.resolve(&selected.profile, algorithm)?;
    let mut syntaxes = ProfileSyntaxRegistry::new();
    syntaxes.register(&selected.profile, resolved.syntax.clone())?;
    let (ast, program) = parse_internal(source, source_name, algorithm, Some(&syntaxes))?;
    finish_compilation(
        ast,
        program,
        source_name,
        algorithm,
        Some(&resolved.effective_policy),
        Some(resolved.type_mode),
    )
}

fn compile_source_internal(
    source: &[u8],
    source_name: &str,
    algorithm: HashAlgorithm,
    policy: Option<&EffectivePolicyDocument>,
    profiles: Option<&ProfileSyntaxRegistry>,
) -> Result<Compilation> {
    let (ast, program) = parse_internal(source, source_name, algorithm, profiles)?;
    finish_compilation(ast, program, source_name, algorithm, policy, None)
}

fn finish_compilation(
    ast: CanonicalAstDocument,
    program: ParsedProgram,
    source_name: &str,
    algorithm: HashAlgorithm,
    policy: Option<&EffectivePolicyDocument>,
    profile_mode: Option<TypeMode>,
) -> Result<Compilation> {
    let mut ir = elaborate(&program, source_name, algorithm)?;
    ir.type_mode = profile_mode.unwrap_or(TypeMode::InferStrict);
    for goal in &mut ir.goals {
        goal.type_mode = ir.type_mode;
    }
    if let Some(policy) = policy {
        apply_effective_policy(&mut ir, policy, source_name, algorithm, profile_mode)?;
    }
    let semantic_hash = semantic_hash_with(&ir, algorithm)?;
    ir.semantic_id = Some(semantic_hash.clone());
    let ir_hash = artifact_hash_with(&ir.to_value(false), algorithm)?;
    ir.artifact_id = Some(ir_hash.clone());
    ir.validate()?;
    let ast_bytes = encode_deterministic(&ast.to_value(true))?;
    let ir_bytes = encode_deterministic(&ir.to_value(true))?;
    let ast_hash = ast.artifact_id.clone().expect("validated AST has identity");
    Ok(Compilation {
        ast_hash,
        ast,
        ir,
        ast_bytes,
        ir_bytes,
        semantic_hash,
        ir_hash,
        effective_policy: policy.cloned(),
    })
}

fn apply_effective_policy(
    ir: &mut SemanticIrDocument,
    policy: &EffectivePolicyDocument,
    source_name: &str,
    algorithm: HashAlgorithm,
    profile_mode: Option<TypeMode>,
) -> Result<()> {
    PolicyDocument::Effective(policy.clone()).validate()?;
    if let Some(feature) = policy.header.features.first() {
        return Err(policy_enforcement_error(
            "BHCP8200",
            format!("unsupported effective policy feature {feature:?}"),
            source_name,
        ));
    }
    let semantic_id = policy.header.semantic_id.clone().ok_or_else(|| {
        policy_enforcement_error(
            "BHCP8200",
            "effective policy requires a semantic identity",
            source_name,
        )
    })?;
    let artifact_id = policy.header.artifact_id.clone().ok_or_else(|| {
        policy_enforcement_error(
            "BHCP8200",
            "effective policy requires an artifact identity",
            source_name,
        )
    })?;
    if semantic_id.algorithm != algorithm.id() || artifact_id.algorithm != algorithm.id() {
        return Err(policy_enforcement_error(
            "BHCP8200",
            "source and effective policy identity algorithms differ",
            source_name,
        ));
    }

    let required_mode = policy.effective.type_mode.value;
    let source_mode = profile_mode.unwrap_or(TypeMode::InferStrict);
    if profile_mode.is_none() && source_mode < required_mode {
        return Err(policy_enforcement_error(
            "BHCP8201",
            format!(
                "source requests type mode {} below effective policy minimum {}",
                source_mode.as_str(),
                required_mode.as_str()
            ),
            source_name,
        ));
    }

    let decision_mode = profile_mode.map_or(required_mode, |mode| mode.max(required_mode));
    ir.type_mode = decision_mode;
    for goal in &mut ir.goals {
        goal.type_mode = decision_mode;
        let requirements =
            applicable_indices(&policy.effective.requirements, &goal.symbol, |rule| {
                rule.value.scope.as_ref()
            });
        let evidence = applicable_indices(&policy.effective.evidence, &goal.symbol, |rule| {
            rule.value.scope.as_ref()
        });
        let prohibitions =
            applicable_indices(&policy.effective.prohibitions, &goal.symbol, |rule| {
                rule.value.scope.as_ref()
            });
        let capabilities =
            applicable_indices(&policy.effective.capabilities, &goal.symbol, |rule| {
                rule.value.scope.as_ref()
            });
        let limits = applicable_indices(&policy.effective.limits, &goal.symbol, |rule| {
            rule.value.scope.as_ref()
        });

        for clause in &goal.clauses {
            match &clause.kind {
                ClauseKind::Authority {
                    kind: "allows",
                    effects,
                } => {
                    for effect in effects {
                        if prohibitions.iter().any(|index| {
                            policy.effective.prohibitions[*index].value.effect == effect.id
                        }) {
                            return Err(policy_enforcement_error(
                                "BHCP8202",
                                format!(
                                    "goal {} requests prohibited effect {}",
                                    goal.symbol, effect.id
                                ),
                                source_name,
                            ));
                        }
                        let granted = capabilities.iter().any(|index| {
                            let capability = &policy.effective.capabilities[*index].value;
                            capability.effect == effect.id
                                && request_within_scope(effect, capability.scope.as_ref())
                        });
                        if !granted {
                            return Err(policy_enforcement_error(
                                "BHCP8203",
                                format!(
                                    "goal {} has unresolved authority for effect {}",
                                    goal.symbol, effect.id
                                ),
                                source_name,
                            ));
                        }
                    }
                }
                ClauseKind::Contract {
                    kind: "limit",
                    dimension: Some(dimension),
                    condition,
                } => {
                    let requested = expression_limit_maximum(condition).ok_or_else(|| {
                        policy_enforcement_error(
                            "BHCP8204",
                            format!(
                                "goal {} limit {} must use a direct non-negative exact upper bound",
                                goal.symbol, dimension
                            ),
                            source_name,
                        )
                    })?;
                    for index in &limits {
                        let ceiling = &policy.effective.limits[*index].value;
                        if ceiling.dimension == *dimension
                            && requested.compare(&ceiling.maximum).is_gt()
                        {
                            return Err(policy_enforcement_error(
                                "BHCP8204",
                                format!(
                                    "goal {} limit {} exceeds the effective policy maximum",
                                    goal.symbol, dimension
                                ),
                                source_name,
                            ));
                        }
                    }
                }
                _ => {}
            }
        }

        goal.policy_decision = Some(PolicyDecision {
            type_mode: decision_mode.as_str().to_owned(),
            requirements,
            evidence,
            prohibitions,
            capabilities,
            limits,
        });
    }
    ir.effective_policy = Some(EffectivePolicyReference {
        semantic_id,
        artifact_id,
    });
    Ok(())
}

fn applicable_indices<T, F>(rules: &[T], goal: &str, scope: F) -> Vec<usize>
where
    F: Fn(&T) -> Option<&PolicyScope>,
{
    rules
        .iter()
        .enumerate()
        .filter_map(|(index, rule)| {
            scope(rule)
                .is_none_or(|scope| scope_matches_goal(scope, goal))
                .then_some(index)
        })
        .collect()
}

fn scope_matches_goal(scope: &PolicyScope, goal: &str) -> bool {
    scope
        .goals
        .as_ref()
        .is_none_or(|goals| goals.iter().any(|candidate| candidate == goal))
}

fn request_within_scope(effect: &Effect, scope: Option<&PolicyScope>) -> bool {
    let Some(scope) = scope else {
        return true;
    };
    let resource_allowed = scope.resources.as_ref().is_none_or(|resources| {
        effect
            .resource
            .as_ref()
            .is_some_and(|resource| resources.contains(resource))
    });
    let operation_allowed = scope.operations.as_ref().is_none_or(|operations| {
        effect.parameters.iter().any(|parameter| {
            matches!(parameter, Value::Text(operation) if operations.contains(operation))
        })
    });
    resource_allowed && operation_allowed
}

fn expression_limit_maximum(expression: &Expression) -> Option<ExactNumber> {
    let ExpressionForm::Binary(operator, _, right) = &expression.form else {
        return None;
    };
    if operator != "<=" {
        return None;
    }
    let ExpressionForm::Literal(value) = &right.form else {
        return None;
    };
    match value {
        Value::Array(parts) => match parts.as_slice() {
            [Value::Text(kind), Value::Integer(value)] if kind == "integer" && *value >= 0 => {
                Some(ExactNumber::Integer(*value))
            }
            [
                Value::Text(kind),
                Value::Integer(numerator),
                Value::Integer(denominator),
            ] if kind == "rational" && *numerator >= 0 && *denominator > 0 => {
                Some(ExactNumber::Rational {
                    numerator: *numerator,
                    denominator: *denominator,
                })
            }
            [
                Value::Text(kind),
                Value::Integer(coefficient),
                Value::Integer(exponent),
            ] if kind == "decimal" && *coefficient >= 0 => Some(ExactNumber::Decimal {
                coefficient: *coefficient,
                exponent: *exponent,
            }),
            _ => None,
        },
        _ => None,
    }
}

fn policy_enforcement_error(
    code: &'static str,
    message: impl Into<String>,
    source_name: &str,
) -> Diagnostic {
    Diagnostic::new(code, message, source_name, 1, 1)
}

fn parse_internal(
    source: &[u8],
    source_name: &str,
    algorithm: HashAlgorithm,
    profiles: Option<&ProfileSyntaxRegistry>,
) -> Result<(CanonicalAstDocument, ParsedProgram)> {
    let selected = scan_profile_preamble(source, source_name)?;
    let bytes = source;
    let source_ref = ContentReference {
        media_type: format!(
            "text/bhcp;profile={}",
            percent_encode_profile(&selected.profile)
        ),
        size: bytes.len(),
        digests: vec![algorithm.hash(bytes)],
    };
    let program = if selected.profile == CANONICAL_PROFILE {
        parse_canonical(&selected.canonical_source, source_name, source_ref.clone())?
    } else if let Some(syntax) = profiles.and_then(|registry| registry.get(&selected.profile)) {
        parse_with_syntax(
            &selected.canonical_source,
            source_name,
            source_ref.clone(),
            syntax,
        )
        .map_err(|diagnostic| attach_profile_context(diagnostic, &selected.profile))?
    } else {
        return Err(Diagnostic::new(
            "BHCP0004",
            format!(
                "selected syntax profile {:?} is not registered for normalization in this slice",
                selected.profile
            ),
            source_name,
            1,
            1,
        ));
    };
    let mut features = features_for(algorithm);
    if uses_self_hosted_all(&program) {
        features.push(ALL_FEATURE.to_owned());
    }
    if uses_self_hosted_any(&program) {
        features.push(ANY_FEATURE.to_owned());
    }
    if uses_self_hosted_none(&program) {
        features.push(NONE_FEATURE.to_owned());
    }
    if uses_self_hosted_chain(&program) {
        features.push(CHAIN_FEATURE.to_owned());
    }
    if uses_self_hosted_gate(&program) {
        features.push(GATE_FEATURE.to_owned());
    }
    if uses_ownership(&program) {
        features.push(OWNERSHIP_FEATURE.to_owned());
    }
    let mut ast = CanonicalAstDocument {
        features,
        profile: selected.profile,
        root: program.ast.clone(),
        source: source_ref,
        artifact_id: None,
    };
    ast.artifact_id = Some(artifact_hash_with(&ast.to_value(false), algorithm)?);
    ast.validate()?;
    Ok((ast, program))
}

fn attach_profile_context(mut diagnostic: Diagnostic, profile: &str) -> Diagnostic {
    if matches!(diagnostic.code, "BHCP9002" | "BHCP0005")
        && !diagnostic.message.starts_with("profile=")
    {
        diagnostic.message = format!("profile={profile} {}", diagnostic.message);
    }
    diagnostic
}

fn percent_encode_profile(profile: &str) -> String {
    let mut encoded = String::with_capacity(profile.len());
    for byte in profile.bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-') {
            encoded.push(char::from(byte));
        } else {
            use std::fmt::Write as _;
            write!(encoded, "%{byte:02X}").expect("writing to a String cannot fail");
        }
    }
    encoded
}

struct Ids {
    counters: HashMap<&'static str, usize>,
}
impl Ids {
    fn new() -> Self {
        Self {
            counters: HashMap::new(),
        }
    }
    fn next(&mut self, kind: &'static str) -> String {
        let counter = self.counters.entry(kind).or_default();
        *counter += 1;
        format!("{kind}-{counter}")
    }
}

fn elaborate(
    program: &ParsedProgram,
    source_name: &str,
    algorithm: HashAlgorithm,
) -> Result<SemanticIrDocument> {
    let checked_types = check_type_definitions(program)?;
    analyze_program(program, source_name)?;
    let types = checked_types.definitions.clone();
    let mut definitions = DefinitionElaborator::new(program, &checked_types, source_name)?;
    definitions.elaborate_roots()?;
    if !program.policies.is_empty()
        || !program.syntaxes.is_empty()
        || !program.profiles.is_empty()
        || !program.waivers.is_empty()
        || !program.extensions.is_empty()
    {
        let at = program
            .policies
            .iter()
            .map(|definition| &definition.at)
            .chain(program.syntaxes.iter().map(|definition| &definition.at))
            .chain(program.profiles.iter().map(|definition| &definition.at))
            .chain(program.waivers.iter().map(|definition| &definition.at))
            .chain(program.extensions.iter().map(|definition| &definition.at))
            .min_by_key(|point| point.byte)
            .expect("governance definition was present");
        return Err(error(
            "BHCP2004",
            "governance definitions lower to typed documents, not executable goal IR",
            source_name,
            at,
        ));
    }
    if program.goals.is_empty()
        && program.types.is_empty()
        && program.functions.is_empty()
        && program.predicates.is_empty()
    {
        return Err(Diagnostic::new(
            "BHCP1001",
            "an executable source file must contain at least one goal",
            source_name,
            1,
            1,
        ));
    }
    if let Some(unsupported) = program
        .goals
        .iter()
        .find_map(|goal| goal.unsupported.as_ref())
    {
        return Err(error(
            "BHCP1004",
            &unsupported.message,
            source_name,
            &unsupported.at,
        ));
    }
    let mut symbols = HashSet::new();
    for goal in &program.goals {
        if !symbols.insert(&goal.symbol) {
            return Err(error(
                "BHCP2002",
                format!("duplicate goal symbol {}", goal.symbol),
                source_name,
                &goal.at,
            ));
        }
    }
    let mut signatures = HashMap::new();
    for (index, goal) in program.goals.iter().enumerate() {
        let signature = goal_signature(goal, index, source_name)?;
        signatures.insert(goal.symbol.clone(), signature);
    }
    resolve_gate_outputs(program, &mut signatures, source_name)?;
    let prelude = Prelude::load()?;
    let mut ids = Ids::new();
    let mut goals = Vec::new();
    let mut functions = Vec::new();
    for (index, goal) in program.goals.iter().enumerate() {
        let mut state = GoalLoweringState {
            definitions: &mut definitions,
            functions: &mut functions,
            ids: &mut ids,
        };
        goals.push(lower_goal(
            goal,
            index,
            source_name,
            &signatures,
            &prelude,
            &mut state,
        )?);
    }
    let (pure_functions, predicates) = definitions.finish();
    let entrypoints = goals.iter().map(|goal| goal.id.clone()).collect();
    let mut features = features_for(algorithm);
    if uses_self_hosted_all(program) {
        features.push(ALL_FEATURE.to_owned());
    }
    if uses_self_hosted_any(program) {
        features.push(ANY_FEATURE.to_owned());
    }
    if uses_self_hosted_none(program) {
        features.push(NONE_FEATURE.to_owned());
    }
    if uses_self_hosted_chain(program) {
        features.push(CHAIN_FEATURE.to_owned());
    }
    if uses_self_hosted_gate(program) {
        features.push(GATE_FEATURE.to_owned());
    }
    if uses_ownership(program) {
        features.push(OWNERSHIP_FEATURE.to_owned());
    }
    Ok(SemanticIrDocument {
        features,
        type_mode: TypeMode::InferStrict,
        types,
        functions,
        pure_functions,
        predicates,
        goals,
        entrypoints,
        effective_policy: None,
        semantic_id: None,
        artifact_id: None,
    })
}

fn uses_self_hosted_all(program: &ParsedProgram) -> bool {
    program.goals.iter().any(|goal| match &goal.body {
        Some(SurfaceComposition::DerivedAll { .. }) => true,
        Some(SurfaceComposition::Compose { reducer, .. }) => reducer == ALL_REDUCER,
        _ => false,
    })
}

fn uses_self_hosted_any(program: &ParsedProgram) -> bool {
    program.goals.iter().any(|goal| match &goal.body {
        Some(SurfaceComposition::DerivedAny { .. }) => true,
        Some(SurfaceComposition::Compose { reducer, .. }) => reducer == ANY_REDUCER,
        _ => false,
    })
}

fn uses_self_hosted_none(program: &ParsedProgram) -> bool {
    program.goals.iter().any(|goal| {
        goal.body
            .as_ref()
            .is_some_and(is_self_hosted_none_composition)
    })
}

fn is_self_hosted_none_composition(composition: &SurfaceComposition) -> bool {
    match composition {
        SurfaceComposition::DerivedNone { .. } => true,
        SurfaceComposition::Compose { reducer, .. } => reducer == NONE_REDUCER,
        _ => false,
    }
}

fn uses_self_hosted_chain(program: &ParsedProgram) -> bool {
    program.goals.iter().any(|goal| {
        goal.body
            .as_ref()
            .is_some_and(is_self_hosted_chain_composition)
    })
}

fn is_self_hosted_chain_composition(composition: &SurfaceComposition) -> bool {
    match composition {
        SurfaceComposition::DerivedChain { .. } => true,
        SurfaceComposition::Compose { reducer, .. } => reducer == CHAIN_REDUCER,
        _ => false,
    }
}

fn uses_self_hosted_gate(program: &ParsedProgram) -> bool {
    program
        .goals
        .iter()
        .any(|goal| matches!(&goal.body, Some(SurfaceComposition::DerivedGate { .. })))
}

fn uses_ownership(program: &ParsedProgram) -> bool {
    program.goals.iter().any(|goal| {
        goal.clauses.iter().any(|clause| {
            matches!(
                &clause.kind,
                SurfaceClauseKind::Fact { value_type, .. }
                    if surface_type_contains_handle(value_type)
            )
        })
    }) || program
        .goals
        .iter()
        .any(|goal| ast_contains_handle(&goal.ast))
        || program
            .types
            .iter()
            .any(|definition| ast_contains_handle(&definition.ast))
        || program
            .functions
            .iter()
            .any(|definition| ast_contains_handle(&definition.ast))
        || program
            .predicates
            .iter()
            .any(|definition| ast_contains_handle(&definition.ast))
}

fn surface_type_contains_handle(value_type: &SurfaceType) -> bool {
    match value_type {
        SurfaceType::Handle { .. } => true,
        SurfaceType::Record(fields) => fields
            .iter()
            .any(|field| surface_type_contains_handle(&field.value_type)),
        SurfaceType::StructuralRecord { fields, .. } => fields
            .iter()
            .any(|field| surface_type_contains_handle(&field.value_type)),
        SurfaceType::Tuple(members) => members.iter().any(surface_type_contains_handle),
        SurfaceType::Variant(cases) => cases
            .iter()
            .flat_map(|case| &case.payload)
            .any(surface_type_contains_handle),
        SurfaceType::List(element)
        | SurfaceType::Set(element)
        | SurfaceType::Option(element)
        | SurfaceType::Reduction(element) => surface_type_contains_handle(element),
        SurfaceType::Map { key, value }
        | SurfaceType::Result {
            ok: key,
            error: value,
        } => surface_type_contains_handle(key) || surface_type_contains_handle(value),
        SurfaceType::Nominal { arguments, .. }
        | SurfaceType::Union(arguments)
        | SurfaceType::Intersection(arguments) => {
            arguments.iter().any(surface_type_contains_handle)
        }
        SurfaceType::Goal {
            input,
            output,
            evidence,
            ..
        } => {
            surface_type_contains_handle(input)
                || surface_type_contains_handle(output)
                || evidence
                    .as_deref()
                    .is_some_and(surface_type_contains_handle)
        }
        SurfaceType::Refined { value_type, .. } => surface_type_contains_handle(value_type),
        SurfaceType::Primitive(_)
        | SurfaceType::Exact(_)
        | SurfaceType::Parameter(_)
        | SurfaceType::Dynamic
        | SurfaceType::Meta { .. }
        | SurfaceType::Never => false,
    }
}

fn ast_contains_handle(node: &crate::model::AstNode) -> bool {
    node.attributes
        .iter()
        .any(|(_, value)| value_contains_handle(value))
        || node.children.iter().any(ast_contains_handle)
}

fn value_contains_handle(value: &Value) -> bool {
    match value {
        Value::Array(values) => {
            values.first() == Some(&Value::Text("handle".to_owned()))
                || values.iter().any(value_contains_handle)
        }
        Value::Map(entries) => entries
            .iter()
            .any(|(_, value)| value_contains_handle(value)),
        Value::Tag(_, value) => value_contains_handle(value),
        _ => false,
    }
}

fn gate_output(child: BhcpType) -> BhcpType {
    BhcpType::Variant(vec![
        VariantCaseType {
            tag: "Excluded".to_owned(),
            payload: vec![],
        },
        VariantCaseType {
            tag: "Included".to_owned(),
            payload: vec![child],
        },
    ])
}

fn resolve_gate_outputs(
    program: &ParsedProgram,
    signatures: &mut HashMap<String, GoalSignature>,
    source_name: &str,
) -> Result<()> {
    let by_symbol: HashMap<_, _> = program
        .goals
        .iter()
        .map(|goal| (goal.symbol.as_str(), goal))
        .collect();
    let symbols: Vec<_> = program
        .goals
        .iter()
        .map(|goal| goal.symbol.clone())
        .collect();
    for symbol in symbols {
        let mut visiting = HashSet::new();
        resolve_gate_output(&symbol, &by_symbol, signatures, source_name, &mut visiting)?;
    }
    Ok(())
}

fn resolve_gate_output(
    symbol: &str,
    goals: &HashMap<&str, &SurfaceGoal>,
    signatures: &mut HashMap<String, GoalSignature>,
    source_name: &str,
    visiting: &mut HashSet<String>,
) -> Result<BhcpType> {
    let goal = goals[symbol];
    let Some(SurfaceComposition::DerivedGate { branches, at, .. }) = &goal.body else {
        return Ok(signatures[symbol].output.clone());
    };
    if goal
        .clauses
        .iter()
        .any(|clause| matches!(clause.kind, SurfaceClauseKind::Fact { kind: "output", .. }))
    {
        return Ok(signatures[symbol].output.clone());
    }
    let [branch] = branches.as_slice() else {
        return Err(error(
            "BHCP2003",
            "gate composition requires exactly one child",
            source_name,
            at,
        ));
    };
    if !visiting.insert(symbol.to_owned()) {
        return Err(error(
            "BHCP2003",
            "gate output inference cannot contain a recursive gate cycle",
            source_name,
            at,
        ));
    }
    let child = goals.get(branch.goal.as_str()).ok_or_else(|| {
        error(
            "BHCP2001",
            format!("unresolved goal {}", branch.goal),
            source_name,
            &branch.at,
        )
    })?;
    let child_output =
        resolve_gate_output(&child.symbol, goals, signatures, source_name, visiting)?;
    visiting.remove(symbol);
    let output = gate_output(child_output);
    signatures.get_mut(symbol).expect("signature exists").output = output.clone();
    Ok(output)
}

fn has_implicit_unit_output(goal: &SurfaceGoal) -> bool {
    goal.body.as_ref().is_some_and(|composition| {
        is_self_hosted_none_composition(composition)
            || (is_self_hosted_chain_composition(composition) && composition.branches().is_empty())
    })
}

#[derive(Clone)]
struct GoalSignature {
    id: String,
    input: BhcpType,
    output: BhcpType,
}

fn goal_signature(goal: &SurfaceGoal, index: usize, source_name: &str) -> Result<GoalSignature> {
    let mut names = HashSet::new();
    let mut input_fields = Vec::new();
    let mut output_fields = Vec::new();
    for clause in &goal.clauses {
        let SurfaceClauseKind::Fact {
            kind,
            name,
            value_type,
        } = &clause.kind
        else {
            continue;
        };
        if !names.insert(name.clone()) {
            return Err(error(
                "BHCP2002",
                format!("duplicate observable name {name:?}"),
                source_name,
                &clause.at,
            ));
        }
        let field = FieldType {
            name: name.clone(),
            value_type: lower_type(value_type, source_name, &clause.at)?,
        };
        if *kind == "input" {
            input_fields.push(field);
        } else {
            output_fields.push(field);
        }
    }
    input_fields.sort_by(|left, right| left.name.cmp(&right.name));
    output_fields.sort_by(|left, right| left.name.cmp(&right.name));
    let output = if output_fields.is_empty() && has_implicit_unit_output(goal) {
        BhcpType::Primitive("Unit")
    } else {
        BhcpType::Record(output_fields)
    };
    Ok(GoalSignature {
        id: format!("goal-{}", index + 1),
        input: BhcpType::Record(input_fields),
        output,
    })
}

struct GoalLoweringState<'a> {
    definitions: &'a mut DefinitionElaborator,
    functions: &'a mut Vec<FunctionDefinition>,
    ids: &'a mut Ids,
}

fn lower_goal(
    goal: &SurfaceGoal,
    index: usize,
    source_name: &str,
    signatures: &HashMap<String, GoalSignature>,
    prelude: &Prelude,
    state: &mut GoalLoweringState<'_>,
) -> Result<GoalDefinition> {
    let GoalLoweringState {
        definitions,
        functions,
        ids,
    } = state;
    let clause_ids: Vec<_> = goal.clauses.iter().map(|_| ids.next("clause")).collect();
    let mut labels = HashMap::new();
    for (index, clause) in goal.clauses.iter().enumerate() {
        let Some(label) = &clause.label else {
            continue;
        };
        if labels
            .insert(label.clone(), (clause_ids[index].clone(), index))
            .is_some()
        {
            return Err(error(
                "BHCP2002",
                format!("duplicate clause label {label:?}"),
                source_name,
                &clause.at,
            ));
        }
    }
    let mut environment = HashMap::new();
    let mut bindings = HashMap::new();
    let mut input_fields = Vec::new();
    let mut output_fields = Vec::new();
    for (clause_index, clause) in goal.clauses.iter().enumerate() {
        if let SurfaceClauseKind::Fact {
            kind,
            name,
            value_type,
        } = &clause.kind
        {
            if environment.contains_key(name) {
                return Err(error(
                    "BHCP2002",
                    format!("duplicate observable name {name:?}"),
                    source_name,
                    &clause.at,
                ));
            }
            let value_type = lower_type(value_type, source_name, &clause.at)?;
            let binding = Binding {
                id: ids.next("binding"),
                name: name.clone(),
                value_type: value_type.clone(),
            };
            environment.insert(name.clone(), binding.clone());
            bindings.insert(clause_index, binding);
            let field = FieldType {
                name: name.clone(),
                value_type,
            };
            if *kind == "input" {
                input_fields.push(field);
            } else {
                output_fields.push(field);
            }
        }
    }
    input_fields.sort_by(|left, right| left.name.cmp(&right.name));
    output_fields.sort_by(|left, right| left.name.cmp(&right.name));
    let input = BhcpType::Record(input_fields);
    let declared_output = BhcpType::Record(output_fields);
    let signature = &signatures[&goal.symbol];
    debug_assert_eq!(signature.input, input);
    debug_assert!(
        signature.output == declared_output
            || (declared_output == BhcpType::Record(vec![]) && has_implicit_unit_output(goal))
            || (declared_output == BhcpType::Record(vec![])
                && matches!(&goal.body, Some(SurfaceComposition::DerivedGate { .. })))
    );
    let output = signature.output.clone();
    let mut clauses = Vec::new();
    for (clause_index, surface) in goal.clauses.iter().enumerate() {
        let kind = match &surface.kind {
            SurfaceClauseKind::Fact { kind, .. } => ClauseKind::Fact {
                kind,
                binding: bindings[&clause_index].clone(),
            },
            SurfaceClauseKind::Contract {
                kind,
                dimension,
                condition,
            } => {
                let condition =
                    lower_expression(condition, &environment, definitions, source_name, ids)?;
                if condition.value_type != BhcpType::Primitive("Bool") {
                    return Err(error(
                        "BHCP2003",
                        format!("{kind} condition must have type Bool"),
                        source_name,
                        &surface.at,
                    ));
                }
                ClauseKind::Contract {
                    kind,
                    dimension: dimension.clone(),
                    condition,
                }
            }
            SurfaceClauseKind::Authority { kind, effects } => {
                let mut effects: Vec<_> = effects
                    .iter()
                    .map(|effect| lower_effect(effect, &environment, source_name))
                    .collect::<Result<_>>()?;
                effects.sort_by(|left, right| left.id.cmp(&right.id));
                ClauseKind::Authority { kind, effects }
            }
            SurfaceClauseKind::Preference {
                priority,
                objective,
            } => ClauseKind::Preference {
                priority: *priority,
                objective: lower_expression(
                    objective,
                    &environment,
                    definitions,
                    source_name,
                    ids,
                )?,
            },
            SurfaceClauseKind::Verify {
                verifier,
                obligation_labels,
            } => {
                let mut obligations = obligation_labels
                    .iter()
                    .map(|label| {
                        let Some((id, target_index)) = labels.get(label) else {
                            return Err(error(
                                "BHCP2001",
                                format!("unresolved obligation label {label:?}"),
                                source_name,
                                &surface.at,
                            ));
                        };
                        if !matches!(
                            goal.clauses[*target_index].kind,
                            SurfaceClauseKind::Contract { .. }
                        ) {
                            return Err(error(
                                "BHCP2003",
                                format!("label {label:?} does not name a contract obligation"),
                                source_name,
                                &surface.at,
                            ));
                        }
                        Ok(id.clone())
                    })
                    .collect::<Result<Vec<_>>>()?;
                obligations.sort();
                obligations.dedup();
                ClauseKind::Verify {
                    binding: VerifierBinding {
                        verifier: verifier.clone(),
                        input: BhcpType::Record(vec![
                            FieldType {
                                name: "input".to_owned(),
                                value_type: input.clone(),
                            },
                            FieldType {
                                name: "output".to_owned(),
                                value_type: output.clone(),
                            },
                        ]),
                        output: BhcpType::Evidence(vec![]),
                        trust: vec![],
                    },
                    obligations,
                }
            }
            SurfaceClauseKind::SyntaxOnly { kind } => {
                return Err(error(
                    "BHCP1004",
                    format!("goal syntax {kind} is outside the implemented executable slice"),
                    source_name,
                    &surface.at,
                ));
            }
        };
        clauses.push(Clause {
            id: clause_ids[clause_index].clone(),
            label: surface.label.clone(),
            kind,
        });
    }
    let evidence = if clauses
        .iter()
        .any(|clause| matches!(clause.kind, ClauseKind::Verify { .. }))
    {
        "static"
    } else {
        "unresolved"
    };
    let body = goal
        .body
        .as_ref()
        .map(|composition| {
            lower_composition(
                composition,
                signature,
                source_name,
                signatures,
                prelude,
                functions,
                ids,
            )
        })
        .transpose()?;
    Ok(GoalDefinition {
        id: format!("goal-{}", index + 1),
        symbol: goal.symbol.clone(),
        type_mode: TypeMode::InferStrict,
        input,
        output,
        evidence: BhcpType::Evidence(vec![evidence.to_owned()]),
        clauses,
        policy_decision: None,
        body,
    })
}

fn lower_composition(
    composition: &SurfaceComposition,
    parent: &GoalSignature,
    source_name: &str,
    signatures: &HashMap<String, GoalSignature>,
    prelude: &Prelude,
    functions: &mut Vec<FunctionDefinition>,
    ids: &mut Ids,
) -> Result<KernelNetwork> {
    let is_chain = is_self_hosted_chain_composition(composition);
    let gate_condition = match composition {
        SurfaceComposition::DerivedGate {
            condition,
            branches,
            ..
        } => {
            if branches.len() != 1 {
                return Err(error(
                    "BHCP2003",
                    "gate composition requires exactly one child",
                    source_name,
                    composition.at(),
                ));
            }
            let condition = lower_gate_condition(condition, &parent.input, source_name, ids)?;
            if condition.value_type != BhcpType::Primitive("Bool") {
                return Err(error(
                    "BHCP2003",
                    "gate condition must have type Bool",
                    source_name,
                    composition.at(),
                ));
            }
            Some(condition)
        }
        _ => None,
    };
    let mut tags = HashSet::new();
    let mut children = Vec::new();
    for branch in composition.branches() {
        if !tags.insert(branch.tag.clone()) {
            return Err(error(
                "BHCP2002",
                format!("duplicate composition tag {:?}", branch.tag),
                source_name,
                &branch.at,
            ));
        }
        let child = signatures.get(&branch.goal).ok_or_else(|| {
            error(
                "BHCP2001",
                format!("unresolved goal {}", branch.goal),
                source_name,
                &branch.at,
            )
        })?;
        let arguments = if is_chain {
            lower_chain_arguments(branch, child, children.last(), source_name, ids)?
        } else if gate_condition.is_some() {
            lower_gate_arguments(branch, child, &parent.input, source_name, ids)?
        } else {
            if child.input != BhcpType::Record(vec![]) || !branch.arguments.is_empty() {
                return Err(error(
                    "BHCP2004",
                    "goal-call arguments are implemented only for chain composition",
                    source_name,
                    &branch.at,
                ));
            }
            vec![]
        };
        children.push(DerivedChild {
            tag: branch.tag.clone(),
            goal: child.id.clone(),
            output: child.output.clone(),
            arguments,
        });
    }
    if !is_chain {
        children.sort_by(|left, right| left.tag.cmp(&right.tag));
    }
    let shape = match composition {
        SurfaceComposition::DerivedAll { .. } => prelude.lower(
            ALL_LOWERER,
            DerivedForm {
                input: parent.input.clone(),
                output: parent.output.clone(),
                children,
                condition: None,
            },
        )?,
        SurfaceComposition::DerivedAny { .. } => prelude.lower(
            ANY_LOWERER,
            DerivedForm {
                input: parent.input.clone(),
                output: parent.output.clone(),
                children,
                condition: None,
            },
        )?,
        SurfaceComposition::DerivedNone { .. } => prelude.lower(
            NONE_LOWERER,
            DerivedForm {
                input: parent.input.clone(),
                output: parent.output.clone(),
                children,
                condition: None,
            },
        )?,
        SurfaceComposition::DerivedChain { .. } => prelude.lower(
            CHAIN_LOWERER,
            DerivedForm {
                input: parent.input.clone(),
                output: parent.output.clone(),
                children,
                condition: None,
            },
        )?,
        SurfaceComposition::DerivedGate { .. } => prelude.lower(
            GATE_LOWERER,
            DerivedForm {
                input: parent.input.clone(),
                output: parent.output.clone(),
                children,
                condition: gate_condition.clone(),
            },
        )?,
        SurfaceComposition::Compose { reducer, .. } => NetworkShape {
            output: parent.output.clone(),
            children,
            reducer: reducer.clone(),
        },
        SurfaceComposition::SyntaxOnly { .. } => {
            return Err(error(
                "BHCP1004",
                "goal syntax is outside the implemented executable slice",
                source_name,
                composition.at(),
            ));
        }
    };
    if shape.output != parent.output {
        return Err(error(
            "BHCP2003",
            "composition output does not match the parent goal output",
            source_name,
            composition.at(),
        ));
    }

    let mut observation_fields = Vec::new();
    for child in &shape.children {
        observation_fields.push(FieldType {
            name: child.tag.clone(),
            value_type: BhcpType::Option(Box::new(BhcpType::ExecutionResult(Box::new(
                child.output.clone(),
            )))),
        });
    }
    observation_fields.sort_by(|left, right| left.name.cmp(&right.name));
    let observations = BhcpType::Record(observation_fields);
    let reducer_source = prelude.reducer(&shape.reducer).map_err(|_| {
        error(
            "BHCP2004",
            format!(
                "network reducer {} is outside the implemented prelude slice",
                shape.reducer
            ),
            source_name,
            composition.at(),
        )
    })?;
    let condition_identity = match composition {
        SurfaceComposition::DerivedGate { condition, .. } => {
            Some(surface_expression_identity(condition)?)
        }
        _ => None,
    };
    let reducer_symbol = specialized_reducer_symbol(
        &shape.reducer,
        &parent.input,
        &observations,
        &parent.output,
        condition_identity.as_ref(),
    )?;
    if !functions
        .iter()
        .any(|function| function.symbol == reducer_symbol)
    {
        functions.push(instantiate_reducer(
            reducer_source,
            reducer_symbol.clone(),
            ReducerSpecialization {
                input: parent.input.clone(),
                observations,
                output: parent.output.clone(),
                gate_condition: gate_condition.as_ref(),
            },
            source_name,
            ids,
        )?);
    }

    Ok(KernelNetwork {
        id: ids.next("network"),
        output: parent.output.clone(),
        children: shape
            .children
            .into_iter()
            .map(|child| KernelChild {
                id: ids.next("child"),
                tag: child.tag,
                goal: child.goal,
                arguments: child.arguments,
            })
            .collect(),
        reducer: reducer_symbol,
    })
}

fn lower_chain_arguments(
    branch: &crate::parser::SurfaceBranch,
    child: &GoalSignature,
    predecessor: Option<&DerivedChild>,
    source_name: &str,
    ids: &mut Ids,
) -> Result<Vec<KernelArgument>> {
    let BhcpType::Record(input_fields) = &child.input else {
        unreachable!("goal inputs are records in the implemented source slice")
    };
    let Some(predecessor) = predecessor else {
        if input_fields.is_empty() && branch.arguments.is_empty() {
            return Ok(vec![]);
        }
        return Err(error(
            "BHCP2003",
            "the first chain child must have no predecessor input",
            source_name,
            &branch.at,
        ));
    };
    let ([field], [argument]) = (input_fields.as_slice(), branch.arguments.as_slice()) else {
        return Err(error(
            "BHCP2003",
            "each later chain child must bind its one input to the immediate predecessor output",
            source_name,
            &branch.at,
        ));
    };
    if argument.name != field.name {
        return Err(error(
            "BHCP2001",
            format!(
                "chain argument {:?} does not name the child input",
                argument.name
            ),
            source_name,
            &argument.at,
        ));
    }
    if argument.source != predecessor.tag {
        return Err(error(
            "BHCP2001",
            format!(
                "chain argument source {:?} is not the immediate predecessor {:?}",
                argument.source, predecessor.tag
            ),
            source_name,
            &argument.at,
        ));
    }
    if !data_edge_type_compatible(
        &predecessor.output,
        &field.value_type,
        kernel_argument_mode(argument.mode),
    ) {
        return Err(error(
            "BHCP2003",
            "chain predecessor output does not match the child input type",
            source_name,
            &argument.at,
        ));
    }
    let tag = Expression {
        id: ids.next("expr"),
        value_type: BhcpType::Primitive("Text"),
        form: ExpressionForm::Literal(Value::Text(predecessor.tag.clone())),
    };
    let value = Expression {
        id: ids.next("expr"),
        value_type: predecessor.output.clone(),
        form: ExpressionForm::Call("bhcp/kernel.observed-output@0".to_owned(), vec![tag]),
    };
    let mode = kernel_argument_mode(argument.mode);
    Ok(vec![KernelArgument {
        name: argument.name.clone(),
        mode,
        value,
    }])
}

fn lower_gate_arguments(
    branch: &crate::parser::SurfaceBranch,
    child: &GoalSignature,
    parent_input: &BhcpType,
    source_name: &str,
    ids: &mut Ids,
) -> Result<Vec<KernelArgument>> {
    let (BhcpType::Record(child_fields), BhcpType::Record(parent_fields)) =
        (&child.input, parent_input)
    else {
        unreachable!("goal inputs are records in the implemented source slice")
    };
    if branch.arguments.len() != child_fields.len() {
        return Err(error(
            "BHCP2003",
            "gate child arguments must exactly cover its typed input fields",
            source_name,
            &branch.at,
        ));
    }
    let mut lowered = Vec::with_capacity(child_fields.len());
    for field in child_fields {
        let Some(argument) = branch
            .arguments
            .iter()
            .find(|argument| argument.name == field.name)
        else {
            return Err(error(
                "BHCP2001",
                format!("gate argument does not name child input {:?}", field.name),
                source_name,
                &branch.at,
            ));
        };
        let Some(parent_field) = parent_fields
            .iter()
            .find(|parent_field| parent_field.name == argument.source)
        else {
            return Err(error(
                "BHCP2001",
                format!(
                    "gate argument source {:?} is not a parent input",
                    argument.source
                ),
                source_name,
                &argument.at,
            ));
        };
        if !data_edge_type_compatible(
            &parent_field.value_type,
            &field.value_type,
            kernel_argument_mode(argument.mode),
        ) {
            return Err(error(
                "BHCP2003",
                "gate parent input does not match the child input type",
                source_name,
                &argument.at,
            ));
        }
        let field_name = Expression {
            id: ids.next("expr"),
            value_type: BhcpType::Primitive("Text"),
            form: ExpressionForm::Literal(Value::Text(argument.source.clone())),
        };
        let value = Expression {
            id: ids.next("expr"),
            value_type: field.value_type.clone(),
            form: ExpressionForm::Call("bhcp/kernel.parent-field@0".to_owned(), vec![field_name]),
        };
        let mode = kernel_argument_mode(argument.mode);
        lowered.push(KernelArgument {
            name: argument.name.clone(),
            mode,
            value,
        });
    }
    Ok(lowered)
}

fn parent_field_type<'a>(parent: &'a BhcpType, name: &str) -> Option<&'a BhcpType> {
    let BhcpType::Record(fields) = parent else {
        return None;
    };
    fields
        .iter()
        .find(|field| field.name == name)
        .map(|field| &field.value_type)
}

fn kernel_argument_mode(mode: SurfaceArgumentMode) -> ArgumentMode {
    match mode {
        SurfaceArgumentMode::Value => ArgumentMode::Value,
        SurfaceArgumentMode::Move => ArgumentMode::Move,
        SurfaceArgumentMode::Borrow => ArgumentMode::Borrow,
        SurfaceArgumentMode::Share => ArgumentMode::Share,
    }
}

fn surface_expression_identity(expression: &SurfaceExpression) -> Result<Value> {
    Ok(match expression {
        SurfaceExpression::Literal { value, .. } => Value::Array(vec![
            Value::Text("literal".to_owned()),
            match value {
                SurfaceLiteral::Bool(value) => Value::Bool(*value),
                SurfaceLiteral::Text(value) => Value::Text(value.clone()),
                SurfaceLiteral::Integer(value) => Value::Array(vec![
                    Value::Text("integer".to_owned()),
                    Value::Integer(i128::from(*value)),
                ]),
            },
        ]),
        SurfaceExpression::Reference { name, .. } => Value::Array(vec![
            Value::Text("parent-field".to_owned()),
            Value::Text(name.clone()),
        ]),
        SurfaceExpression::Unary {
            operator, operand, ..
        } => Value::Array(vec![
            Value::Text("unary".to_owned()),
            Value::Text(operator.clone()),
            surface_expression_identity(operand)?,
        ]),
        SurfaceExpression::Binary {
            operator,
            left,
            right,
            ..
        } => Value::Array(vec![
            Value::Text("binary".to_owned()),
            Value::Text(operator.clone()),
            surface_expression_identity(left)?,
            surface_expression_identity(right)?,
        ]),
        SurfaceExpression::If {
            condition,
            consequent,
            alternative,
            ..
        } => Value::Array(vec![
            Value::Text("if".to_owned()),
            surface_expression_identity(condition)?,
            surface_expression_identity(consequent)?,
            surface_expression_identity(alternative)?,
        ]),
        SurfaceExpression::Call {
            function,
            arguments,
            ..
        } => Value::Array(vec![
            Value::Text("call".to_owned()),
            Value::Text(function.clone()),
            Value::Array(
                arguments
                    .iter()
                    .map(surface_expression_identity)
                    .collect::<Result<Vec<_>>>()?,
            ),
        ]),
    })
}

fn lower_gate_condition(
    surface: &SurfaceExpression,
    parent: &BhcpType,
    source_name: &str,
    ids: &mut Ids,
) -> Result<Expression> {
    let (value_type, form) = match surface {
        SurfaceExpression::Literal { value, .. } => match value {
            SurfaceLiteral::Bool(value) => (
                BhcpType::Primitive("Bool"),
                ExpressionForm::Literal(Value::Bool(*value)),
            ),
            SurfaceLiteral::Text(value) => (
                BhcpType::Primitive("Text"),
                ExpressionForm::Literal(Value::Text(value.clone())),
            ),
            SurfaceLiteral::Integer(value) => (
                BhcpType::ExactNumber("Integer"),
                ExpressionForm::Literal(Value::Array(vec![
                    Value::Text("integer".to_owned()),
                    Value::Integer(i128::from(*value)),
                ])),
            ),
        },
        SurfaceExpression::Reference { name, at } => {
            let value_type = parent_field_type(parent, name).cloned().ok_or_else(|| {
                error(
                    "BHCP2003",
                    format!("unresolved gate-condition input {name:?}"),
                    source_name,
                    at,
                )
            })?;
            let field = Expression {
                id: ids.next("expr"),
                value_type: BhcpType::Primitive("Text"),
                form: ExpressionForm::Literal(Value::Text(name.clone())),
            };
            (
                value_type,
                ExpressionForm::Call("bhcp/kernel.parent-field@0".to_owned(), vec![field]),
            )
        }
        SurfaceExpression::Unary {
            operator,
            operand,
            at,
        } => {
            let operand = lower_gate_condition(operand, parent, source_name, ids)?;
            if operator != "!" || operand.value_type != BhcpType::Primitive("Bool") {
                return Err(error(
                    "BHCP2003",
                    "gate unary condition must preserve Bool",
                    source_name,
                    at,
                ));
            }
            (
                BhcpType::Primitive("Bool"),
                ExpressionForm::Unary(operator.clone(), Box::new(operand)),
            )
        }
        SurfaceExpression::Binary {
            operator,
            left,
            right,
            at,
        } => {
            let left = lower_gate_condition(left, parent, source_name, ids)?;
            let right = lower_gate_condition(right, parent, source_name, ids)?;
            let valid = match operator.as_str() {
                "==" | "!=" => left.value_type == right.value_type,
                "&&" | "||" => {
                    left.value_type == BhcpType::Primitive("Bool")
                        && right.value_type == BhcpType::Primitive("Bool")
                }
                _ => false,
            };
            if !valid {
                return Err(error(
                    "BHCP2003",
                    "gate binary condition is not total and consistently typed",
                    source_name,
                    at,
                ));
            }
            (
                BhcpType::Primitive("Bool"),
                ExpressionForm::Binary(operator.clone(), Box::new(left), Box::new(right)),
            )
        }
        SurfaceExpression::If {
            condition,
            consequent,
            alternative,
            at,
        } => {
            let condition = lower_gate_condition(condition, parent, source_name, ids)?;
            let consequent = lower_gate_condition(consequent, parent, source_name, ids)?;
            let alternative = lower_gate_condition(alternative, parent, source_name, ids)?;
            if condition.value_type != BhcpType::Primitive("Bool")
                || consequent.value_type != alternative.value_type
            {
                return Err(error(
                    "BHCP2003",
                    "gate conditional is not total and consistently typed",
                    source_name,
                    at,
                ));
            }
            let value_type = consequent.value_type.clone();
            (
                value_type,
                ExpressionForm::If(
                    Box::new(condition),
                    Box::new(consequent),
                    Box::new(alternative),
                ),
            )
        }
        SurfaceExpression::Call { at, .. } => {
            return Err(error(
                "BHCP2004",
                "gate condition calls are outside the implemented total-pure slice",
                source_name,
                at,
            ));
        }
    };
    Ok(Expression {
        id: ids.next("expr"),
        value_type,
        form,
    })
}

fn gate_type_matches(observations: &BhcpType, output: &BhcpType) -> bool {
    let BhcpType::Record(fields) = observations else {
        return false;
    };
    let [field] = fields.as_slice() else {
        return false;
    };
    let BhcpType::Option(result) = &field.value_type else {
        return false;
    };
    let BhcpType::ExecutionResult(child_output) = result.as_ref() else {
        return false;
    };
    output == &gate_output(child_output.as_ref().clone())
}

fn specialized_reducer_symbol(
    base: &str,
    input: &BhcpType,
    observations: &BhcpType,
    output: &BhcpType,
    condition: Option<&Value>,
) -> Result<String> {
    let mut signature = vec![
        Value::Text(base.to_owned()),
        input.to_value(),
        observations.to_value(),
        output.to_value(),
    ];
    if let Some(condition) = condition {
        signature.push(condition.clone());
    }
    let signature = Value::Array(signature);
    let bytes = encode_deterministic(&signature)?;
    let digest = HashAlgorithm::default().hash(&bytes).digest;
    let suffix: String = digest[..16]
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    let (path, version) = base.rsplit_once('@').ok_or_else(|| {
        Diagnostic::plain(
            "BHCP3001",
            "prelude reducer is not a versioned semantic name",
        )
    })?;
    Ok(format!("{path}-{suffix}@{version}"))
}

struct ReducerSpecialization<'a> {
    input: BhcpType,
    observations: BhcpType,
    output: BhcpType,
    gate_condition: Option<&'a Expression>,
}

fn instantiate_reducer(
    source: &SurfaceFunction,
    symbol: String,
    specialization: ReducerSpecialization<'_>,
    source_name: &str,
    ids: &mut Ids,
) -> Result<FunctionDefinition> {
    let ReducerSpecialization {
        input,
        observations,
        output,
        gate_condition,
    } = specialization;
    let parent = Binding {
        id: ids.next("parameter"),
        name: source.parameters[0].name.clone(),
        value_type: input,
    };
    let observed = Binding {
        id: ids.next("parameter"),
        name: source.parameters[1].name.clone(),
        value_type: observations,
    };
    let environment = HashMap::from([
        (parent.name.clone(), parent.clone()),
        (observed.name.clone(), observed.clone()),
    ]);
    let result = BhcpType::Reduction(Box::new(output));
    let definition = lower_reducer_expression(
        &source.definition,
        &environment,
        &result,
        gate_condition,
        source_name,
        ids,
    )?;
    if definition.value_type != result {
        return Err(Diagnostic::plain(
            "BHCP3001",
            "specialized reducer body does not match its result type",
        ));
    }
    Ok(FunctionDefinition {
        id: ids.next("function"),
        symbol,
        parameters: vec![parent, observed],
        result,
        definition,
    })
}

fn lower_reducer_expression(
    surface: &SurfaceExpression,
    environment: &HashMap<String, Binding>,
    result_type: &BhcpType,
    gate_condition: Option<&Expression>,
    source_name: &str,
    ids: &mut Ids,
) -> Result<Expression> {
    let (value_type, form) = match surface {
        SurfaceExpression::Literal { value, .. } => match value {
            SurfaceLiteral::Bool(value) => (
                BhcpType::Primitive("Bool"),
                ExpressionForm::Literal(Value::Bool(*value)),
            ),
            SurfaceLiteral::Text(value) => (
                BhcpType::Primitive("Text"),
                ExpressionForm::Literal(Value::Text(value.clone())),
            ),
            SurfaceLiteral::Integer(value) => (
                BhcpType::ExactNumber("Integer"),
                ExpressionForm::Literal(Value::Array(vec![
                    Value::Text("integer".to_owned()),
                    Value::Integer(i128::from(*value)),
                ])),
            ),
        },
        SurfaceExpression::Reference { name, at } => {
            let binding = environment.get(name).ok_or_else(|| {
                error(
                    "BHCP3001",
                    format!("unresolved prelude binding {name:?}"),
                    source_name,
                    at,
                )
            })?;
            (
                binding.value_type.clone(),
                ExpressionForm::Reference(binding.id.clone()),
            )
        }
        SurfaceExpression::Unary {
            operator,
            operand,
            at,
        } => {
            let operand = lower_reducer_expression(
                operand,
                environment,
                result_type,
                gate_condition,
                source_name,
                ids,
            )?;
            if operator != "!" || operand.value_type != BhcpType::Primitive("Bool") {
                return Err(error(
                    "BHCP3001",
                    format!("unsupported or ill-typed pure reducer unary operation {operator}"),
                    source_name,
                    at,
                ));
            }
            (
                BhcpType::Primitive("Bool"),
                ExpressionForm::Unary(operator.clone(), Box::new(operand)),
            )
        }
        SurfaceExpression::Binary {
            operator,
            left,
            right,
            at,
        } => {
            let left = lower_reducer_expression(
                left,
                environment,
                result_type,
                gate_condition,
                source_name,
                ids,
            )?;
            let right = lower_reducer_expression(
                right,
                environment,
                result_type,
                gate_condition,
                source_name,
                ids,
            )?;
            let valid = match operator.as_str() {
                "==" | "!=" => {
                    left.value_type == right.value_type
                        && environment
                            .get("observations")
                            .is_none_or(|binding| binding.value_type != left.value_type)
                }
                "&&" | "||" => {
                    left.value_type == BhcpType::Primitive("Bool")
                        && right.value_type == BhcpType::Primitive("Bool")
                }
                _ => false,
            };
            if !valid {
                return Err(error(
                    "BHCP3001",
                    format!("unsupported or ill-typed pure reducer binary operation {operator}"),
                    source_name,
                    at,
                ));
            }
            (
                BhcpType::Primitive("Bool"),
                ExpressionForm::Binary(operator.clone(), Box::new(left), Box::new(right)),
            )
        }
        SurfaceExpression::Call {
            function,
            arguments,
            at,
        } => {
            if function == "bhcp/prelude.gate-condition@0" {
                let Some(condition) = gate_condition else {
                    return Err(error(
                        "BHCP3001",
                        "gate-condition placeholder is valid only in a gate reducer specialization",
                        source_name,
                        at,
                    ));
                };
                if arguments.len() != 1 {
                    return Err(error(
                        "BHCP3001",
                        "gate-condition placeholder requires the parent parameter",
                        source_name,
                        at,
                    ));
                }
                return Ok(condition.clone());
            }
            let arguments = arguments
                .iter()
                .map(|argument| {
                    lower_reducer_expression(
                        argument,
                        environment,
                        result_type,
                        gate_condition,
                        source_name,
                        ids,
                    )
                })
                .collect::<Result<Vec<_>>>()?;
            let observations_type = environment
                .get("observations")
                .map(|binding| &binding.value_type)
                .ok_or_else(|| Diagnostic::plain("BHCP3001", "missing observations parameter"))?;
            let BhcpType::Reduction(output_type) = result_type else {
                return Err(Diagnostic::plain(
                    "BHCP3001",
                    "reducer specialization requires a Reduction result",
                ));
            };
            let refs_type = BhcpType::List(Box::new(BhcpType::Primitive("Text")));
            let fault_type = BhcpType::Nominal("bhcp/kernel.fault@0".to_owned(), vec![]);
            let reason_type = BhcpType::Nominal("bhcp/kernel.reason@0".to_owned(), vec![]);
            let execution_type = BhcpType::ExecutionResult(output_type.clone());
            let argument_types: Vec<_> = arguments
                .iter()
                .map(|argument| argument.value_type.clone())
                .collect();
            let value_type = match function.as_str() {
                "bhcp/kernel.has-refuted@0"
                | "bhcp/kernel.has-missing@0"
                | "bhcp/kernel.has-faulted@0"
                | "bhcp/kernel.has-unresolved@0"
                | "bhcp/kernel.has-satisfied@0"
                | "bhcp/kernel.all-refuted@0"
                    if argument_types == [observations_type.clone()] =>
                {
                    BhcpType::Primitive("Bool")
                }
                "bhcp/kernel.missing-tags@0"
                | "bhcp/kernel.first-missing-tag@0"
                | "bhcp/kernel.first-counter-evidence@0"
                | "bhcp/kernel.all-counter-evidence@0"
                | "bhcp/kernel.first-satisfied-evidence@0"
                | "bhcp/kernel.partial-evidence@0"
                | "bhcp/kernel.satisfied-evidence@0"
                    if argument_types == [observations_type.clone()] =>
                {
                    refs_type.clone()
                }
                "bhcp/kernel.first-fault@0" if argument_types == [observations_type.clone()] => {
                    fault_type.clone()
                }
                "bhcp/kernel.first-unresolved-reason@0"
                    if argument_types == [observations_type.clone()] =>
                {
                    reason_type.clone()
                }
                "bhcp/kernel.satisfied-record@0"
                | "bhcp/kernel.first-satisfied-output@0"
                | "bhcp/kernel.last-satisfied-output@0"
                | "bhcp/kernel.last-satisfied-output-or-unit@0"
                    if argument_types == [observations_type.clone()] =>
                {
                    output_type.as_ref().clone()
                }
                "bhcp/kernel.first-satisfied-winner@0"
                    if argument_types == [observations_type.clone()]
                        && winner_type_matches(observations_type, output_type) =>
                {
                    output_type.as_ref().clone()
                }
                "bhcp/kernel.included@0"
                    if argument_types == [observations_type.clone()]
                        && gate_type_matches(observations_type, output_type) =>
                {
                    output_type.as_ref().clone()
                }
                "bhcp/kernel.excluded@0"
                    if argument_types == [observations_type.clone()]
                        && gate_type_matches(observations_type, output_type) =>
                {
                    output_type.as_ref().clone()
                }
                "bhcp/kernel.unit@0" if argument_types.is_empty() => BhcpType::Primitive("Unit"),
                "bhcp/kernel.pending@0" if argument_types == [refs_type.clone()] => {
                    result_type.clone()
                }
                "bhcp/kernel.refuted@0" if argument_types == [refs_type.clone()] => {
                    execution_type.clone()
                }
                "bhcp/kernel.faulted@0" if argument_types == [fault_type] => execution_type.clone(),
                "bhcp/kernel.unresolved@0"
                    if argument_types == [reason_type, refs_type.clone()] =>
                {
                    execution_type.clone()
                }
                "bhcp/kernel.satisfied@0"
                    if argument_types == [output_type.as_ref().clone(), refs_type] =>
                {
                    execution_type.clone()
                }
                "bhcp/kernel.conclude@0" if argument_types == [execution_type] => {
                    result_type.clone()
                }
                _ => {
                    return Err(error(
                        "BHCP3001",
                        format!("unregistered or ill-typed pure kernel primitive {function}"),
                        source_name,
                        at,
                    ));
                }
            };
            (
                value_type,
                ExpressionForm::Call(function.clone(), arguments),
            )
        }
        SurfaceExpression::If {
            condition,
            consequent,
            alternative,
            at,
        } => {
            let condition = lower_reducer_expression(
                condition,
                environment,
                result_type,
                gate_condition,
                source_name,
                ids,
            )?;
            let consequent = lower_reducer_expression(
                consequent,
                environment,
                result_type,
                gate_condition,
                source_name,
                ids,
            )?;
            let alternative = lower_reducer_expression(
                alternative,
                environment,
                result_type,
                gate_condition,
                source_name,
                ids,
            )?;
            if condition.value_type != BhcpType::Primitive("Bool")
                || consequent.value_type != alternative.value_type
            {
                return Err(error(
                    "BHCP3001",
                    "prelude if expression is not total and consistently typed",
                    source_name,
                    at,
                ));
            }
            let value_type = consequent.value_type.clone();
            (
                value_type,
                ExpressionForm::If(
                    Box::new(condition),
                    Box::new(consequent),
                    Box::new(alternative),
                ),
            )
        }
    };
    Ok(Expression {
        id: ids.next("expr"),
        value_type,
        form,
    })
}

fn winner_type_matches(observations: &BhcpType, output: &BhcpType) -> bool {
    let BhcpType::Record(fields) = observations else {
        return false;
    };
    let mut child_outputs = fields.iter().map(|field| match &field.value_type {
        BhcpType::Option(result) => match result.as_ref() {
            BhcpType::ExecutionResult(output) => Some(output.as_ref()),
            _ => None,
        },
        _ => None,
    });
    let Some(first) = child_outputs.next() else {
        return true;
    };
    let Some(first) = first else {
        return false;
    };
    if child_outputs.any(|candidate| candidate != Some(first)) {
        return false;
    }
    output
        == &BhcpType::Record(vec![
            FieldType {
                name: "output".to_owned(),
                value_type: first.clone(),
            },
            FieldType {
                name: "tag".to_owned(),
                value_type: BhcpType::Primitive("Text"),
            },
        ])
}

fn lower_type(
    value: &SurfaceType,
    source_name: &str,
    at: &crate::model::Point,
) -> Result<BhcpType> {
    Ok(match value {
        SurfaceType::Primitive(name) => BhcpType::Primitive(name),
        SurfaceType::Exact(name) => BhcpType::ExactNumber(name),
        SurfaceType::Record(fields) => {
            let mut lowered = fields
                .iter()
                .map(|field| {
                    Ok(FieldType {
                        name: field.name.clone(),
                        value_type: lower_type(&field.value_type, source_name, at)?,
                    })
                })
                .collect::<Result<Vec<_>>>()?;
            lowered.sort_by(|left, right| left.name.cmp(&right.name));
            BhcpType::Record(lowered)
        }
        SurfaceType::Reduction(output) => {
            BhcpType::Reduction(Box::new(lower_type(output, source_name, at)?))
        }
        SurfaceType::Nominal { symbol, arguments } => BhcpType::Nominal(
            symbol.clone(),
            arguments
                .iter()
                .map(|argument| lower_type(argument, source_name, at))
                .collect::<Result<Vec<_>>>()?,
        ),
        SurfaceType::Handle {
            ownership,
            access,
            usage,
            lifetime,
            value_type,
        } => BhcpType::Handle(Box::new(HandleType {
            ownership: ownership.clone(),
            access: access.clone().unwrap_or_else(|| {
                if ownership == "shared" {
                    "read"
                } else {
                    "write"
                }
                .to_owned()
            }),
            usage: usage.clone().unwrap_or_else(|| "unrestricted".to_owned()),
            lifetime: lifetime.clone().unwrap_or_else(|| "goal".to_owned()),
            value_type: lower_type(value_type, source_name, at)?,
        })),
        SurfaceType::Parameter(_)
        | SurfaceType::Dynamic
        | SurfaceType::Meta { .. }
        | SurfaceType::Never
        | SurfaceType::StructuralRecord { .. }
        | SurfaceType::Tuple(_)
        | SurfaceType::List(_)
        | SurfaceType::Set(_)
        | SurfaceType::Map { .. }
        | SurfaceType::Option(_)
        | SurfaceType::Result { .. }
        | SurfaceType::Variant(_)
        | SurfaceType::Goal { .. }
        | SurfaceType::Union(_)
        | SurfaceType::Intersection(_)
        | SurfaceType::Refined { .. } => {
            return Err(error(
                "BHCP2004",
                "compile-time and generic types are not permitted in executable goal facts",
                source_name,
                at,
            ));
        }
    })
}

fn lower_expression(
    surface: &SurfaceExpression,
    environment: &HashMap<String, Binding>,
    definitions: &mut DefinitionElaborator,
    source_name: &str,
    ids: &mut Ids,
) -> Result<Expression> {
    let (value_type, form) = match surface {
        SurfaceExpression::Literal { value, .. } => match value {
            SurfaceLiteral::Bool(value) => (
                BhcpType::Primitive("Bool"),
                ExpressionForm::Literal(Value::Bool(*value)),
            ),
            SurfaceLiteral::Text(value) => (
                BhcpType::Primitive("Text"),
                ExpressionForm::Literal(Value::Text(value.clone())),
            ),
            SurfaceLiteral::Integer(value) => (
                BhcpType::ExactNumber("Integer"),
                ExpressionForm::Literal(Value::Array(vec![
                    Value::Text("integer".to_owned()),
                    Value::Integer(i128::from(*value)),
                ])),
            ),
        },
        SurfaceExpression::Reference { name, at } => {
            let binding = environment.get(name).ok_or_else(|| {
                error(
                    "BHCP2001",
                    format!("unresolved name {name:?}"),
                    source_name,
                    at,
                )
            })?;
            (
                binding.value_type.clone(),
                ExpressionForm::Reference(binding.id.clone()),
            )
        }
        SurfaceExpression::Unary {
            operator,
            operand,
            at,
        } => {
            let operand = lower_expression(operand, environment, definitions, source_name, ids)?;
            let accepted = (operator == "!" && operand.value_type == BhcpType::Primitive("Bool"))
                || (operator == "-" && operand.value_type == BhcpType::ExactNumber("Integer"));
            if !accepted {
                return Err(error(
                    "BHCP2003",
                    format!("operator {operator} has an incompatible operand"),
                    source_name,
                    at,
                ));
            }
            let value_type = operand.value_type.clone();
            (
                value_type,
                ExpressionForm::Unary(operator.clone(), Box::new(operand)),
            )
        }
        SurfaceExpression::Binary {
            operator,
            left,
            right,
            at,
        } => {
            let left = lower_expression(left, environment, definitions, source_name, ids)?;
            let right = lower_expression(right, environment, definitions, source_name, ids)?;
            if left.value_type != right.value_type {
                return Err(error(
                    "BHCP2003",
                    format!("operator {operator} has incompatible operand types"),
                    source_name,
                    at,
                ));
            }
            let value_type = match operator.as_str() {
                "==" | "!=" => BhcpType::Primitive("Bool"),
                "<" | "<=" | ">" | ">=" if left.value_type == BhcpType::ExactNumber("Integer") => {
                    BhcpType::Primitive("Bool")
                }
                "+" if matches!(
                    left.value_type,
                    BhcpType::Primitive("Text") | BhcpType::ExactNumber("Integer")
                ) =>
                {
                    left.value_type.clone()
                }
                "&&" | "||" if left.value_type == BhcpType::Primitive("Bool") => {
                    BhcpType::Primitive("Bool")
                }
                "-" | "*" | "/" | "%" if left.value_type == BhcpType::ExactNumber("Integer") => {
                    left.value_type.clone()
                }
                _ => {
                    return Err(error(
                        "BHCP2003",
                        format!("operator {operator} has incompatible operand types"),
                        source_name,
                        at,
                    ));
                }
            };
            (
                value_type,
                ExpressionForm::Binary(operator.clone(), Box::new(left), Box::new(right)),
            )
        }
        SurfaceExpression::Call {
            function,
            arguments,
            at,
        } => {
            let arguments = arguments
                .iter()
                .map(|argument| {
                    lower_expression(argument, environment, definitions, source_name, ids)
                })
                .collect::<Result<Vec<_>>>()?;
            let argument_types = arguments
                .iter()
                .map(|argument| CheckedType::from_value(&argument.value_type.to_value()))
                .collect::<Result<Vec<_>>>()?;
            let resolved = definitions.resolve_call(function, &argument_types, at)?;
            let value_type = bhcp_type_from_checked(&resolved.result, source_name, at)?;
            (value_type, ExpressionForm::Call(resolved.symbol, arguments))
        }
        SurfaceExpression::If {
            condition,
            consequent,
            alternative,
            at,
        } => {
            let condition =
                lower_expression(condition, environment, definitions, source_name, ids)?;
            if condition.value_type != BhcpType::Primitive("Bool") {
                return Err(error(
                    "BHCP2003",
                    "conditional expression requires a Bool condition",
                    source_name,
                    at,
                ));
            }
            let consequent =
                lower_expression(consequent, environment, definitions, source_name, ids)?;
            let alternative =
                lower_expression(alternative, environment, definitions, source_name, ids)?;
            if consequent.value_type != alternative.value_type {
                return Err(error(
                    "BHCP2003",
                    "conditional expression branches have incompatible types",
                    source_name,
                    at,
                ));
            }
            (
                consequent.value_type.clone(),
                ExpressionForm::If(
                    Box::new(condition),
                    Box::new(consequent),
                    Box::new(alternative),
                ),
            )
        }
    };
    Ok(Expression {
        id: ids.next("expr"),
        value_type,
        form,
    })
}

fn lower_effect(
    surface: &SurfaceEffect,
    environment: &HashMap<String, Binding>,
    source_name: &str,
) -> Result<Effect> {
    let mut resource = None;
    let mut parameters = Vec::new();
    for argument in &surface.arguments {
        match argument {
            SurfaceExpression::Reference { name, at } => {
                let binding = environment.get(name).ok_or_else(|| {
                    error(
                        "BHCP2001",
                        format!("unresolved name {name:?}"),
                        source_name,
                        at,
                    )
                })?;
                if resource.is_some() {
                    return Err(error(
                        "BHCP2004",
                        "an effect atom supports one resource reference in this slice",
                        source_name,
                        at,
                    ));
                }
                resource = Some(binding.id.clone());
            }
            SurfaceExpression::Literal { value, .. } => parameters.push(match value {
                SurfaceLiteral::Bool(value) => Value::Bool(*value),
                SurfaceLiteral::Text(value) => Value::Text(value.clone()),
                SurfaceLiteral::Integer(value) => Value::Array(vec![
                    Value::Text("integer".to_owned()),
                    Value::Integer(i128::from(*value)),
                ]),
            }),
            _ => {
                return Err(error(
                    "BHCP2004",
                    "effect parameters must be literals or direct resource references in this slice",
                    source_name,
                    surface.arguments[0].at(),
                ));
            }
        }
    }
    Ok(Effect {
        id: surface.symbol.clone(),
        resource,
        parameters,
    })
}

fn bhcp_type_from_checked(
    value: &CheckedType,
    source_name: &str,
    at: &crate::model::Point,
) -> Result<BhcpType> {
    fn decode(value: &Value) -> Option<BhcpType> {
        let Value::Array(parts) = value else {
            return None;
        };
        match parts.as_slice() {
            [Value::Text(tag), Value::Text(name)] if tag == "primitive" => {
                let name = match name.as_str() {
                    "Bool" => "Bool",
                    "Text" => "Text",
                    "Bytes" => "Bytes",
                    "Unit" => "Unit",
                    "Timestamp" => "Timestamp",
                    "Duration" => "Duration",
                    _ => return None,
                };
                Some(BhcpType::Primitive(name))
            }
            [Value::Text(tag), Value::Text(name)] if tag == "exact-number" => {
                let name = match name.as_str() {
                    "Integer" => "Integer",
                    "Rational" => "Rational",
                    "Decimal" => "Decimal",
                    _ => return None,
                };
                Some(BhcpType::ExactNumber(name))
            }
            [
                Value::Text(tag),
                Value::Text(symbol),
                Value::Array(arguments),
            ] if tag == "nominal" => Some(BhcpType::Nominal(
                symbol.clone(),
                arguments.iter().map(decode).collect::<Option<_>>()?,
            )),
            [Value::Text(tag), element] if tag == "list" => {
                Some(BhcpType::List(Box::new(decode(element)?)))
            }
            [Value::Text(tag), element] if tag == "option" => {
                Some(BhcpType::Option(Box::new(decode(element)?)))
            }
            [Value::Text(tag), output] if tag == "verdict" => {
                Some(BhcpType::Verdict(Box::new(decode(output)?)))
            }
            [Value::Text(tag), output] if tag == "execution-result" => {
                Some(BhcpType::ExecutionResult(Box::new(decode(output)?)))
            }
            [Value::Text(tag), output] if tag == "reduction" => {
                Some(BhcpType::Reduction(Box::new(decode(output)?)))
            }
            [Value::Text(tag), Value::Array(classes)] if tag == "evidence" => {
                Some(BhcpType::Evidence(
                    classes
                        .iter()
                        .map(|value| match value {
                            Value::Text(value) => Some(value.clone()),
                            _ => None,
                        })
                        .collect::<Option<_>>()?,
                ))
            }
            [Value::Text(tag), Value::Bool(false), Value::Array(fields)] if tag == "record" => {
                let fields = fields
                    .iter()
                    .map(|field| match field {
                        Value::Array(parts) => match parts.as_slice() {
                            [Value::Text(name), value_type, Value::Bool(false)] => {
                                Some(FieldType {
                                    name: name.clone(),
                                    value_type: decode(value_type)?,
                                })
                            }
                            _ => None,
                        },
                        _ => None,
                    })
                    .collect::<Option<_>>()?;
                Some(BhcpType::Record(fields))
            }
            [Value::Text(tag), Value::Array(cases)] if tag == "variant" => {
                let cases = cases
                    .iter()
                    .map(|case| match case {
                        Value::Array(parts) => match parts.as_slice() {
                            [Value::Text(name), Value::Array(payload)] => Some(VariantCaseType {
                                tag: name.clone(),
                                payload: payload.iter().map(decode).collect::<Option<_>>()?,
                            }),
                            _ => None,
                        },
                        _ => None,
                    })
                    .collect::<Option<_>>()?;
                Some(BhcpType::Variant(cases))
            }
            _ => None,
        }
    }

    decode(&value.to_value()).ok_or_else(|| {
        error(
            "BHCP2004",
            "checked pure-definition result type is outside the executable goal slice",
            source_name,
            at,
        )
    })
}

fn error(
    code: &'static str,
    message: impl Into<String>,
    source: &str,
    point: &crate::model::Point,
) -> Diagnostic {
    Diagnostic::new(code, message, source, point.line, point.column)
}
