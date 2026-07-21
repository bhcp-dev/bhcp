use std::collections::{BTreeMap, HashMap, HashSet};

use crate::cbor::encode_deterministic;
use crate::definition::{DefinitionElaborator, substitute_checked_type};
use crate::diagnostic::{Diagnostic, Result};
use crate::effects::{
    EFFECT_ANALYSIS_FEATURE, analyze as analyze_effects, canonicalize as canonicalize_effects,
};
use crate::extensions::ExtensionRegistry;
use crate::hash::{HashAlgorithm, artifact_hash_with, semantic_hash_with};
use crate::kernel::{ArgumentMode, KernelArgument, KernelChild, KernelNetwork, RecursionBound};
use crate::model::{
    BhcpType, Binding, CanonicalAstDocument, Clause, ClauseKind, ContentReference, Effect,
    EffectRow, EffectivePolicyReference, Expression, ExpressionForm, ExtensionNode, FieldType,
    FunctionDefinition, GoalDefinition, HandleType, HashId, Point, PolicyDecision,
    SemanticIrDocument, VariantCaseType, VerifierBinding, bhcp_type_contains_handle,
    closed_binary_result_type, closed_unary_result_type, data_edge_type_compatible, features_for,
    is_symbol,
};
use crate::ownership::analyze_program;
use crate::parser::{
    CANONICAL_PROFILE, ParsedProgram, SurfaceArgumentMode, SurfaceClauseKind, SurfaceComposition,
    SurfaceEffect, SurfaceExpression, SurfaceExtension, SurfaceExtensionKind, SurfaceFunction,
    SurfaceGoal, SurfaceLiteral, SurfaceType, parse_canonical, parse_with_syntax,
    scan_profile_preamble, validate_effective_syntax,
};
use crate::policy::{
    EffectivePolicyDocument, ExactNumber, PolicyDocument, PolicyScope, SourcePolicyDocument,
    TypeMode, apply_waiver, compose_policies,
};
use crate::prelude::{
    ALL_FEATURE, ALL_LOWERER, ALL_REDUCER, ANY_FEATURE, ANY_LOWERER, ANY_REDUCER, CHAIN_FEATURE,
    CHAIN_LOWERER, CHAIN_REDUCER, DerivedChild, DerivedForm, GATE_FEATURE, GATE_LOWERER,
    NONE_FEATURE, NONE_LOWERER, NONE_REDUCER, NetworkShape, Prelude, RECURSIVE_GATE_REDUCER,
    RETAIN_FEATURE, RETAIN_REDUCER,
};
use crate::profile::{PresentationDocument, ProfileDocument, ProfileRegistry, SyntaxDocument};
use crate::typecheck::{CheckedType, TypeRelations, check_type_definitions, surface_type};
use crate::value::Value;

const OWNERSHIP_FEATURE: &str = "bhcp/feature.ownership-analysis@0";
const EXTENSION_FEATURE: &str = "bhcp/feature.extension-resolution@0";
pub const PROFILE_SOURCE_FEATURE: &str = "bhcp/feature.profile-source-lowering@0";

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

#[derive(Clone, Debug)]
pub struct ParsedProfileSource {
    pub ast: CanonicalAstDocument,
    pub syntaxes: Vec<SyntaxDocument>,
    pub profiles: Vec<ProfileDocument>,
    pub policies: Vec<SourcePolicyDocument>,
    pub registry: ProfileRegistry,
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

pub fn parse_profile_source(source: &str, source_name: &str) -> Result<ParsedProfileSource> {
    parse_profile_source_with_algorithm(source, source_name, HashAlgorithm::default())
}

pub fn parse_profile_source_with_algorithm(
    source: &str,
    source_name: &str,
    algorithm: HashAlgorithm,
) -> Result<ParsedProfileSource> {
    parse_profile_source_bytes_with_algorithm(source.as_bytes(), source_name, algorithm)
}

pub fn parse_profile_source_bytes(source: &[u8], source_name: &str) -> Result<ParsedProfileSource> {
    parse_profile_source_bytes_with_algorithm(source, source_name, HashAlgorithm::default())
}

pub fn parse_profile_source_bytes_with_algorithm(
    source: &[u8],
    source_name: &str,
    algorithm: HashAlgorithm,
) -> Result<ParsedProfileSource> {
    let (ast, program) = parse_internal(source, source_name, algorithm, None)?;
    if program.syntaxes.is_empty() && program.profiles.is_empty() {
        return Err(Diagnostic::new(
            "BHCP9004",
            "profile source must contain at least one §syntax or §profile definition",
            source_name,
            1,
            1,
        ));
    }
    if !program.types.is_empty()
        || !program.functions.is_empty()
        || !program.predicates.is_empty()
        || !program.refinements.is_empty()
        || !program.goals.is_empty()
        || !program.waivers.is_empty()
        || !program.extensions.is_empty()
    {
        return Err(Diagnostic::new(
            "BHCP9004",
            "profile source may contain only policy, syntax, and profile definitions",
            source_name,
            1,
            1,
        ));
    }

    let mut syntaxes = program
        .syntaxes
        .into_iter()
        .map(|surface| surface.document)
        .collect::<Vec<_>>();
    let mut profiles = program
        .profiles
        .into_iter()
        .map(|surface| surface.document)
        .collect::<Vec<_>>();
    for syntax in &mut syntaxes {
        syntax.header.artifact_id = Some(artifact_hash_with(&syntax.to_value(false), algorithm)?);
        PresentationDocument::Syntax(syntax.clone()).validate()?;
    }
    for profile in &mut profiles {
        profile.header.artifact_id = Some(artifact_hash_with(&profile.to_value(false), algorithm)?);
        PresentationDocument::Profile(profile.clone()).validate()?;
    }
    let policies = program
        .policies
        .into_iter()
        .map(|surface| surface.document)
        .collect::<Vec<_>>();

    let mut registry = ProfileRegistry::new();
    for syntax in &syntaxes {
        registry.register_syntax(syntax.clone())?;
    }
    for profile in &profiles {
        registry.register_profile(profile.clone())?;
    }
    for policy in &policies {
        registry.register_policy(policy.clone())?;
    }
    registry.validate(algorithm)?;

    Ok(ParsedProfileSource {
        ast,
        syntaxes,
        profiles,
        policies,
        registry,
    })
}

pub fn compile_source(source: &str, source_name: &str) -> Result<Compilation> {
    compile_source_with_algorithm(source, source_name, HashAlgorithm::default())
}

pub fn compile_source_with_extension_registry(
    source: &str,
    source_name: &str,
    extensions: &ExtensionRegistry,
) -> Result<Compilation> {
    compile_source_internal(
        source.as_bytes(),
        source_name,
        HashAlgorithm::default(),
        None,
        None,
        Some(extensions),
        None,
    )
}

pub fn compile_source_with_waiver_decision_time(
    source: &str,
    source_name: &str,
    decision_time: &str,
) -> Result<Compilation> {
    compile_source_internal(
        source.as_bytes(),
        source_name,
        HashAlgorithm::default(),
        None,
        None,
        None,
        Some(decision_time),
    )
}

pub fn compile_source_with_extension_registry_and_waiver_decision_time(
    source: &str,
    source_name: &str,
    extensions: &ExtensionRegistry,
    decision_time: &str,
) -> Result<Compilation> {
    compile_source_internal(
        source.as_bytes(),
        source_name,
        HashAlgorithm::default(),
        None,
        None,
        Some(extensions),
        Some(decision_time),
    )
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
        None,
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
    compile_source_internal(source, source_name, algorithm, None, None, None, None)
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
    compile_source_internal(
        source,
        source_name,
        algorithm,
        None,
        Some(profiles),
        None,
        None,
    )
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
        return compile_source_internal(source, source_name, algorithm, None, None, None, None);
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
        CompilationContext {
            policy: Some(&resolved.effective_policy),
            profile_mode: Some(resolved.type_mode),
            extensions: None,
            waiver_decision_time: None,
        },
    )
}

fn compile_source_internal(
    source: &[u8],
    source_name: &str,
    algorithm: HashAlgorithm,
    policy: Option<&EffectivePolicyDocument>,
    profiles: Option<&ProfileSyntaxRegistry>,
    extensions: Option<&ExtensionRegistry>,
    waiver_decision_time: Option<&str>,
) -> Result<Compilation> {
    let (ast, program) = parse_internal(source, source_name, algorithm, profiles)?;
    finish_compilation(
        ast,
        program,
        source_name,
        algorithm,
        CompilationContext {
            policy,
            profile_mode: None,
            extensions,
            waiver_decision_time,
        },
    )
}

struct CompilationContext<'a> {
    policy: Option<&'a EffectivePolicyDocument>,
    profile_mode: Option<TypeMode>,
    extensions: Option<&'a ExtensionRegistry>,
    waiver_decision_time: Option<&'a str>,
}

fn finish_compilation(
    ast: CanonicalAstDocument,
    program: ParsedProgram,
    source_name: &str,
    algorithm: HashAlgorithm,
    context: CompilationContext<'_>,
) -> Result<Compilation> {
    let CompilationContext {
        policy,
        profile_mode,
        extensions,
        waiver_decision_time,
    } = context;
    if policy.is_some() && !program.policies.is_empty() {
        return Err(Diagnostic::new(
            "BHCP8110",
            "inline source policy cannot be combined with a separately supplied effective policy",
            source_name,
            1,
            1,
        ));
    }
    let inline_policy =
        inline_effective_policy(&program, source_name, algorithm, waiver_decision_time)?;
    let policy = policy.or(inline_policy.as_ref());
    let mut ir = elaborate(&program, source_name, algorithm, extensions)?;
    ir.type_mode = profile_mode.unwrap_or(TypeMode::InferStrict);
    for goal in &mut ir.goals {
        goal.type_mode = ir.type_mode;
    }
    analyze_effects(&mut ir.goals, source_name)?;
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

fn inline_effective_policy(
    program: &ParsedProgram,
    source_name: &str,
    algorithm: HashAlgorithm,
    waiver_decision_time: Option<&str>,
) -> Result<Option<EffectivePolicyDocument>> {
    if !program.syntaxes.is_empty()
        || !program.profiles.is_empty()
        || (program.goals.is_empty()
            && program.types.is_empty()
            && program.functions.is_empty()
            && program.predicates.is_empty()
            && program.extensions.is_empty())
    {
        return Ok(None);
    }
    if program.policies.is_empty() {
        if let Some(waiver) = program.waivers.first() {
            return Err(Diagnostic::new(
                "BHCP8301",
                "inline source waiver requires at least one inline source policy",
                source_name,
                waiver.at.line,
                waiver.at.column,
            ));
        }
        return Ok(None);
    }
    let sources = program
        .policies
        .iter()
        .map(|policy| policy.document.clone())
        .collect::<Vec<_>>();
    let mut effective = compose_policies(&sources, algorithm)?;
    if program.waivers.is_empty() {
        return Ok(Some(effective));
    }
    let decision_time = waiver_decision_time.ok_or_else(|| {
        let at = &program.waivers[0].at;
        Diagnostic::new(
            "BHCP8301",
            "inline source waiver requires an explicitly injected decision time",
            source_name,
            at.line,
            at.column,
        )
    })?;
    let mut waivers = program.waivers.iter().collect::<Vec<_>>();
    waivers.sort_by(|left, right| left.symbol.cmp(&right.symbol));
    for waiver in waivers {
        let document = waiver.document.as_ref().ok_or_else(|| {
            Diagnostic::new(
                "BHCP8301",
                "source waiver contains unresolved symbolic artifact references",
                source_name,
                waiver.at.line,
                waiver.at.column,
            )
        })?;
        effective = apply_waiver(&effective, document, decision_time, algorithm)?;
    }
    Ok(Some(effective))
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
    let resource_symbols = binding_resource_symbols(&ir.goals);
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

        for effect in &goal.effects.effects {
            if prohibitions.iter().any(|index| {
                let prohibition = &policy.effective.prohibitions[*index].value;
                prohibition.effect == effect.id
                    && request_within_scope(effect, prohibition.scope.as_ref(), &resource_symbols)
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
                    && request_within_scope(effect, capability.scope.as_ref(), &resource_symbols)
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

        for clause in &goal.clauses {
            if let ClauseKind::Contract {
                kind: "limit",
                dimension: Some(dimension),
                condition,
            } = &clause.kind
            {
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

fn binding_resource_symbols(goals: &[GoalDefinition]) -> HashMap<String, String> {
    goals
        .iter()
        .flat_map(|goal| &goal.clauses)
        .filter_map(|clause| match &clause.kind {
            ClauseKind::Fact { binding, .. } => resource_symbol(&binding.value_type)
                .map(|symbol| (binding.id.clone(), symbol.to_owned())),
            _ => None,
        })
        .collect()
}

fn resource_symbol(value_type: &BhcpType) -> Option<&str> {
    match value_type {
        BhcpType::Handle(handle) => resource_symbol(&handle.value_type),
        BhcpType::Nominal(symbol, _) => Some(symbol),
        _ => None,
    }
}

fn request_within_scope(
    effect: &Effect,
    scope: Option<&PolicyScope>,
    resource_symbols: &HashMap<String, String>,
) -> bool {
    let Some(scope) = scope else {
        return true;
    };
    let resource_allowed = scope.resources.as_ref().is_none_or(|resources| {
        effect
            .resource
            .as_ref()
            .and_then(|resource| resource_symbols.get(resource))
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
    if uses_retention_lowering(&program) {
        features.push(RETAIN_FEATURE.to_owned());
    }
    if uses_ownership(&program) {
        features.push(OWNERSHIP_FEATURE.to_owned());
    }
    if uses_effect_analysis(&program) {
        features.push(EFFECT_ANALYSIS_FEATURE.to_owned());
    }
    if !program.extensions.is_empty() {
        features.push(EXTENSION_FEATURE.to_owned());
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
    extension_registry: Option<&ExtensionRegistry>,
) -> Result<SemanticIrDocument> {
    let checked_types = check_type_definitions(program)?;
    analyze_program(program, source_name)?;
    let types = checked_types.definitions.clone();
    let mut definitions = DefinitionElaborator::new(program, &checked_types, source_name)?;
    definitions.elaborate_roots()?;
    if !program.syntaxes.is_empty() || !program.profiles.is_empty() {
        let at = program
            .syntaxes
            .iter()
            .map(|definition| &definition.at)
            .chain(program.profiles.iter().map(|definition| &definition.at))
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
        && program.extensions.is_empty()
    {
        if !program.policies.is_empty() || !program.waivers.is_empty() {
            let at = program
                .policies
                .iter()
                .map(|definition| &definition.at)
                .chain(program.waivers.iter().map(|definition| &definition.at))
                .min_by_key(|point| point.byte)
                .expect("inline governance definition was present");
            return Err(error(
                "BHCP2004",
                "governance definitions lower to typed documents, not executable goal IR",
                source_name,
                at,
            ));
        }
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
    let prelude = Prelude::load()?.with_project_functions(program)?;
    let mut ids = Ids::new();
    let mut goals = Vec::new();
    let mut functions = Vec::new();
    let extension_specs = derived_extension_specs(program, source_name)?;
    for (index, spec) in extension_specs.iter().enumerate() {
        if !symbols.insert(&spec.symbol) || signatures.contains_key(&spec.symbol) {
            return Err(extension_error(
                format!("duplicate extension semantic symbol {:?}", spec.symbol),
                source_name,
                &spec.at,
            ));
        }
        signatures.insert(
            spec.symbol.clone(),
            GoalSignature {
                id: format!("extension-goal-{}", index + 1),
                input: spec.input.clone(),
                output: spec.output.clone(),
            },
        );
    }
    for (index, spec) in extension_specs.iter().enumerate() {
        let mut state = ReducerLoweringState {
            functions: &mut functions,
            ids: &mut ids,
            type_relations: &checked_types.relations,
        };
        goals.push(lower_derived_extension(
            spec,
            index,
            source_name,
            &signatures,
            &prelude,
            &mut state,
        )?);
    }
    for (index, goal) in program.goals.iter().enumerate() {
        let mut state = GoalLoweringState {
            definitions: &mut definitions,
            functions: &mut functions,
            ids: &mut ids,
            type_relations: &checked_types.relations,
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
    let (mut pure_functions, predicates) = definitions.finish();
    let executed_lowerers = extension_specs
        .iter()
        .map(|spec| spec.lowering.as_str())
        .collect::<HashSet<_>>();
    pure_functions.retain(|function| !executed_lowerers.contains(function.symbol.as_str()));
    let entrypoints = program
        .goals
        .iter()
        .map(|goal| signatures[&goal.symbol].id.clone())
        .collect();
    let extensions = native_extension_nodes(program, extension_registry, source_name, &mut ids)?;
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
    if uses_retention_lowering(program) {
        features.push(RETAIN_FEATURE.to_owned());
    }
    if uses_ownership(program) {
        features.push(OWNERSHIP_FEATURE.to_owned());
    }
    if uses_effect_analysis(program) {
        features.push(EFFECT_ANALYSIS_FEATURE.to_owned());
    }
    if !program.extensions.is_empty() {
        features.push(EXTENSION_FEATURE.to_owned());
    }
    Ok(SemanticIrDocument {
        features,
        type_mode: TypeMode::InferStrict,
        types,
        functions,
        pure_functions,
        predicates,
        goals,
        extensions,
        entrypoints,
        effective_policy: None,
        semantic_id: None,
        artifact_id: None,
    })
}

#[derive(Clone)]
struct DerivedExtensionSpec {
    symbol: String,
    lowering: String,
    input: BhcpType,
    output: BhcpType,
    children: Vec<String>,
    at: Point,
}

fn derived_extension_specs(
    program: &ParsedProgram,
    source_name: &str,
) -> Result<Vec<DerivedExtensionSpec>> {
    let mut specs = program
        .extensions
        .iter()
        .filter(|extension| extension.extension_kind == SurfaceExtensionKind::Derived)
        .map(|extension| derived_extension_spec(extension, program, source_name))
        .collect::<Result<Vec<_>>>()?;
    specs.sort_by(|left, right| left.symbol.cmp(&right.symbol));
    for pair in specs.windows(2) {
        if pair[0].symbol == pair[1].symbol {
            return Err(extension_error(
                format!("duplicate derived extension {:?}", pair[0].symbol),
                source_name,
                &pair[1].at,
            ));
        }
    }
    Ok(specs)
}

fn derived_extension_spec(
    extension: &SurfaceExtension,
    program: &ParsedProgram,
    source_name: &str,
) -> Result<DerivedExtensionSpec> {
    if extension.symbol.starts_with("bhcp/") {
        return Err(extension_error(
            "extension cannot override a core semantic name",
            source_name,
            &extension.at,
        ));
    }
    let lowering = extension_field_text(extension, "lowering").ok_or_else(|| {
        extension_error(
            "derived extension is missing its lowering function",
            source_name,
            &extension.at,
        )
    })?;
    let function = program
        .functions
        .iter()
        .find(|function| function.symbol == lowering)
        .ok_or_else(|| {
            extension_error(
                format!("derived extension lowering {lowering:?} does not resolve"),
                source_name,
                &extension.at,
            )
        })?;
    if !function.type_parameters.is_empty() || function.parameters.len() != 1 {
        return Err(extension_error(
            "derived extension lowering must be one concrete total pure function",
            source_name,
            &function.at,
        ));
    }
    let SurfaceType::Meta {
        kind: "derived-form",
        input,
        output,
    } = &function.parameters[0].value_type
    else {
        return Err(extension_error(
            "derived extension lowering input must be Meta<DerivedForm,I,O>",
            source_name,
            &function.at,
        ));
    };
    let SurfaceType::Meta {
        kind: "network-shape",
        input: result_input,
        output: result_output,
    } = &function.result
    else {
        return Err(extension_error(
            "derived extension lowering result must be Meta<NetworkShape,I,O>",
            source_name,
            &function.at,
        ));
    };
    let input = lower_type(input, source_name, &function.at)?;
    let output = lower_type(output, source_name, &function.at)?;
    if input != lower_type(result_input, source_name, &function.at)?
        || output != lower_type(result_output, source_name, &function.at)?
    {
        return Err(extension_error(
            "derived extension lowering meta input and result types differ",
            source_name,
            &function.at,
        ));
    }
    if let Some(declared) = extension_field_text(extension, "input")
        && input != legacy_extension_type(declared, source_name, &extension.at)?
    {
        return Err(extension_error(
            "derived extension input does not match its lowering signature",
            source_name,
            &extension.at,
        ));
    }
    if let Some(declared) = extension_field_text(extension, "output")
        && output != legacy_extension_type(declared, source_name, &extension.at)?
    {
        return Err(extension_error(
            "derived extension output does not match its lowering signature",
            source_name,
            &extension.at,
        ));
    }
    let mut children = extension
        .fields
        .iter()
        .find(|field| field.name == "children")
        .map(|field| match &field.value {
            Value::Array(values) => values
                .iter()
                .map(|value| match value {
                    Value::Text(symbol) if is_symbol(symbol) => Ok(symbol.clone()),
                    _ => Err(extension_error(
                        "derived extension child is not an exact symbol-id",
                        source_name,
                        &field.at,
                    )),
                })
                .collect::<Result<Vec<_>>>(),
            _ => Err(extension_error(
                "derived extension children are not an array",
                source_name,
                &field.at,
            )),
        })
        .transpose()?
        .unwrap_or_default();
    children.sort();
    if children.windows(2).any(|pair| pair[0] == pair[1]) {
        return Err(extension_error(
            "derived extension children must be unique",
            source_name,
            &extension.at,
        ));
    }
    Ok(DerivedExtensionSpec {
        symbol: extension.symbol.clone(),
        lowering: lowering.to_owned(),
        input,
        output,
        children,
        at: extension.at.clone(),
    })
}

fn extension_field_text<'a>(extension: &'a SurfaceExtension, name: &str) -> Option<&'a str> {
    extension
        .fields
        .iter()
        .find(|field| field.name == name)
        .and_then(|field| match &field.value {
            Value::Text(value) => Some(value.as_str()),
            _ => None,
        })
}

fn legacy_extension_type(value: &str, source_name: &str, at: &Point) -> Result<BhcpType> {
    let value_type = match value {
        "Bool" | "Bytes" | "Duration" | "Text" | "Timestamp" | "Unit" => {
            BhcpType::Primitive(match value {
                "Bool" => "Bool",
                "Bytes" => "Bytes",
                "Duration" => "Duration",
                "Text" => "Text",
                "Timestamp" => "Timestamp",
                "Unit" => "Unit",
                _ => unreachable!(),
            })
        }
        "Decimal" | "Integer" | "Rational" => BhcpType::ExactNumber(match value {
            "Decimal" => "Decimal",
            "Integer" => "Integer",
            "Rational" => "Rational",
            _ => unreachable!(),
        }),
        symbol if is_symbol(symbol) => BhcpType::Nominal(symbol.to_owned(), vec![]),
        _ => {
            return Err(extension_error(
                "legacy extension type is not canonical",
                source_name,
                at,
            ));
        }
    };
    Ok(value_type)
}

fn lower_derived_extension(
    spec: &DerivedExtensionSpec,
    index: usize,
    source_name: &str,
    signatures: &HashMap<String, GoalSignature>,
    prelude: &Prelude,
    state: &mut ReducerLoweringState<'_>,
) -> Result<GoalDefinition> {
    let ReducerLoweringState {
        functions,
        ids,
        type_relations,
    } = state;
    let signature = signatures
        .get(&spec.symbol)
        .expect("derived extension signature was indexed");
    let children = spec
        .children
        .iter()
        .enumerate()
        .map(|(child_index, symbol)| {
            let child = signatures.get(symbol).ok_or_else(|| {
                extension_error(
                    format!("derived extension child {symbol:?} does not resolve"),
                    source_name,
                    &spec.at,
                )
            })?;
            Ok(DerivedChild {
                tag: format!("extension-child-{}", child_index + 1),
                goal: child.id.clone(),
                output: child.output.clone(),
                arguments: vec![],
                recursion: None,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    let shape = prelude
        .lower(
            &spec.lowering,
            DerivedForm {
                input: spec.input.clone(),
                output: spec.output.clone(),
                children,
                condition: None,
            },
        )
        .map_err(|diagnostic| {
            extension_error(
                format!("derived extension lowering failed: {}", diagnostic.message),
                source_name,
                &spec.at,
            )
        })?;
    if shape.output != spec.output {
        return Err(extension_error(
            "derived extension lowering returned the wrong output type",
            source_name,
            &spec.at,
        ));
    }
    let mut observation_fields = shape
        .children
        .iter()
        .map(|child| FieldType {
            name: child.tag.clone(),
            value_type: BhcpType::Option(Box::new(BhcpType::ExecutionResult(Box::new(
                child.output.clone(),
            )))),
        })
        .collect::<Vec<_>>();
    observation_fields.sort_by(|left, right| left.name.cmp(&right.name));
    let observations = BhcpType::Record(observation_fields);
    let reducer_source = prelude.reducer(&shape.reducer).map_err(|_| {
        extension_error(
            format!(
                "derived extension reducer {:?} does not resolve",
                shape.reducer
            ),
            source_name,
            &spec.at,
        )
    })?;
    if !reducer_signature_matches(
        reducer_source,
        &spec.input,
        &observations,
        &spec.output,
        type_relations,
    ) {
        return Err(extension_error(
            "derived extension reducer signature does not match its specialization",
            source_name,
            &spec.at,
        ));
    }
    let reducer_symbol = specialized_reducer_symbol(
        &shape.reducer,
        &spec.input,
        &observations,
        &spec.output,
        None,
    )?;
    if !functions
        .iter()
        .any(|function| function.symbol == reducer_symbol)
    {
        functions.push(instantiate_reducer(
            reducer_source,
            reducer_symbol.clone(),
            ReducerSpecialization {
                input: spec.input.clone(),
                observations,
                output: spec.output.clone(),
                gate_condition: None,
            },
            source_name,
            type_relations,
            ids,
        )?);
    }
    let body = KernelNetwork {
        id: ids.next("network"),
        output: spec.output.clone(),
        children: shape
            .children
            .into_iter()
            .map(|child| KernelChild {
                id: ids.next("child"),
                tag: child.tag,
                goal: child.goal,
                arguments: child.arguments,
                recursion: None,
            })
            .collect(),
        reducer: reducer_symbol,
    };
    Ok(GoalDefinition {
        id: format!("extension-goal-{}", index + 1),
        symbol: spec.symbol.clone(),
        type_mode: TypeMode::InferStrict,
        input: signature.input.clone(),
        output: signature.output.clone(),
        effects: EffectRow::empty(),
        evidence: BhcpType::Evidence(vec!["unresolved".to_owned()]),
        clauses: vec![],
        policy_decision: None,
        body: Some(body),
    })
}

fn native_extension_nodes(
    program: &ParsedProgram,
    registry: Option<&ExtensionRegistry>,
    source_name: &str,
    ids: &mut Ids,
) -> Result<Vec<ExtensionNode>> {
    let mut native = program
        .extensions
        .iter()
        .filter(|extension| extension.extension_kind == SurfaceExtensionKind::Native)
        .collect::<Vec<_>>();
    native.sort_by(|left, right| left.symbol.cmp(&right.symbol));
    let mut nodes = Vec::with_capacity(native.len());
    for extension in native {
        if extension.symbol.starts_with("bhcp/") {
            return Err(extension_error(
                "extension cannot override a core semantic name",
                source_name,
                &extension.at,
            ));
        }
        let registration = registry
            .and_then(|registry| registry.native(&extension.symbol))
            .ok_or_else(|| {
                extension_error(
                    format!("unsupported native extension {:?}", extension.symbol),
                    source_name,
                    &extension.at,
                )
            })?;
        let descriptor = extension.descriptor.as_ref().ok_or_else(|| {
            extension_error(
                "native extension descriptor was not materialized",
                source_name,
                &extension.at,
            )
        })?;
        if descriptor.get("payload_schema") != Some(&registration.payload_schema) {
            return Err(extension_error(
                "native extension payload schema does not match its registration",
                source_name,
                &extension.at,
            ));
        }
        nodes.push(ExtensionNode {
            id: ids.next("extension"),
            extension: extension.symbol.clone(),
            payload: Value::map([
                ("descriptor", descriptor.clone()),
                ("value", registration.payload.clone()),
            ]),
            must_understand: true,
        });
    }
    nodes.sort_by(|left, right| {
        encode_deterministic(&left.to_value())
            .expect("typed extension node encodes")
            .cmp(&encode_deterministic(&right.to_value()).expect("typed extension node encodes"))
    });
    Ok(nodes)
}

fn extension_error(message: impl Into<String>, source_name: &str, at: &Point) -> Diagnostic {
    Diagnostic::new("BHCP5003", message, source_name, at.line, at.column)
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

fn uses_retention_lowering(program: &ParsedProgram) -> bool {
    program.goals.iter().any(|goal| {
        matches!(
            &goal.body,
            Some(SurfaceComposition::Compose { reducer, .. }) if reducer == RETAIN_REDUCER
        )
    })
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

fn uses_effect_analysis(program: &ParsedProgram) -> bool {
    program.goals.iter().any(|goal| {
        goal.clauses.iter().any(|clause| {
            matches!(
                clause.kind,
                SurfaceClauseKind::Authority { .. }
                    | SurfaceClauseKind::Contract {
                        kind: "limit",
                        dimension: Some(_),
                        ..
                    }
                    | SurfaceClauseKind::Preference { .. }
            )
        })
    })
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
    if branch.goal == symbol {
        visiting.remove(symbol);
        let output = BhcpType::Primitive("Unit");
        signatures.get_mut(symbol).expect("signature exists").output = output.clone();
        return Ok(output);
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
    type_relations: &'a TypeRelations,
}

struct ReducerLoweringState<'a> {
    functions: &'a mut Vec<FunctionDefinition>,
    ids: &'a mut Ids,
    type_relations: &'a TypeRelations,
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
        type_relations,
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
                let effects: Vec<_> = effects
                    .iter()
                    .map(|effect| lower_effect(effect, &environment, source_name))
                    .collect::<Result<_>>()?;
                ClauseKind::Authority {
                    kind,
                    effects: canonicalize_effects(effects)?,
                }
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
            let mut state = ReducerLoweringState {
                functions,
                ids,
                type_relations,
            };
            lower_composition(
                composition,
                signature,
                source_name,
                signatures,
                prelude,
                &goal.clauses,
                &mut state,
            )
        })
        .transpose()?;
    Ok(GoalDefinition {
        id: format!("goal-{}", index + 1),
        symbol: goal.symbol.clone(),
        type_mode: TypeMode::InferStrict,
        input,
        output,
        effects: EffectRow::empty(),
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
    parent_clauses: &[crate::parser::SurfaceClause],
    state: &mut ReducerLoweringState<'_>,
) -> Result<KernelNetwork> {
    let ReducerLoweringState {
        functions,
        ids,
        type_relations,
    } = state;
    let is_chain = is_self_hosted_chain_composition(composition);
    let is_retention = matches!(
        composition,
        SurfaceComposition::Compose { reducer, .. } if reducer == RETAIN_REDUCER
    );
    let is_sequential = is_chain || is_retention;
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
        let arguments = if is_retention {
            lower_retention_arguments(branch, child, &parent.input, &children, source_name, ids)?
        } else if is_sequential {
            lower_chain_arguments(branch, child, children.last(), source_name, ids)?
        } else if gate_condition.is_some() {
            lower_gate_arguments(branch, child, &parent.input, source_name, ids)?
        } else {
            lower_parent_arguments(branch, child, &parent.input, source_name, ids)?
        };
        let recursion = if child.id == parent.id {
            Some(check_recursion_bound(
                branch,
                &arguments,
                parent_clauses,
                match composition {
                    SurfaceComposition::DerivedGate { condition, .. } => Some(condition),
                    _ => None,
                },
                source_name,
            )?)
        } else {
            None
        };
        children.push(DerivedChild {
            tag: branch.tag.clone(),
            goal: child.id.clone(),
            output: child.output.clone(),
            arguments,
            recursion,
        });
    }
    if !is_sequential {
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
        SurfaceComposition::DerivedGate { .. }
            if children.iter().any(|child| child.goal == parent.id) =>
        {
            if parent.output != BhcpType::Primitive("Unit") {
                return Err(error(
                    "BHCP2003",
                    "recursive gate requires Unit output in the implemented finite slice",
                    source_name,
                    composition.at(),
                ));
            }
            NetworkShape {
                output: parent.output.clone(),
                children,
                reducer: RECURSIVE_GATE_REDUCER.to_owned(),
            }
        }
        SurfaceComposition::DerivedGate { .. } => prelude.lower(
            GATE_LOWERER,
            DerivedForm {
                input: parent.input.clone(),
                output: parent.output.clone(),
                children,
                condition: gate_condition.clone(),
            },
        )?,
        SurfaceComposition::Compose { reducer, .. } if reducer == RETAIN_REDUCER => prelude
            .lower_retention(DerivedForm {
                input: parent.input.clone(),
                output: parent.output.clone(),
                children,
                condition: None,
            })?,
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
            type_relations,
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
                recursion: child.recursion,
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
        return Err(error(
            "BHCP2003",
            "chain children must expose record inputs",
            source_name,
            &branch.at,
        ));
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

fn lower_parent_arguments(
    branch: &crate::parser::SurfaceBranch,
    child: &GoalSignature,
    parent_input: &BhcpType,
    source_name: &str,
    ids: &mut Ids,
) -> Result<Vec<KernelArgument>> {
    let BhcpType::Record(child_fields) = &child.input else {
        return Err(error(
            "BHCP2003",
            "composition children must expose record inputs",
            source_name,
            &branch.at,
        ));
    };
    if branch.arguments.len() != child_fields.len() {
        return Err(error(
            "BHCP2003",
            "composition child arguments must exactly cover its typed input fields",
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
                format!(
                    "composition argument does not name child input {:?}",
                    field.name
                ),
                source_name,
                &branch.at,
            ));
        };
        let value = lower_gate_condition(&argument.value, parent_input, source_name, ids)?;
        if !data_edge_type_compatible(
            &value.value_type,
            &field.value_type,
            kernel_argument_mode(argument.mode),
        ) {
            return Err(error(
                "BHCP2003",
                "composition argument expression does not match the child input type",
                source_name,
                &argument.at,
            ));
        }
        lowered.push(KernelArgument {
            name: argument.name.clone(),
            mode: kernel_argument_mode(argument.mode),
            value,
        });
    }
    lowered.sort_by(|left, right| left.name.cmp(&right.name));
    Ok(lowered)
}

fn lower_retention_arguments(
    branch: &crate::parser::SurfaceBranch,
    child: &GoalSignature,
    parent_input: &BhcpType,
    predecessors: &[DerivedChild],
    source_name: &str,
    ids: &mut Ids,
) -> Result<Vec<KernelArgument>> {
    let BhcpType::Record(child_fields) = &child.input else {
        return Err(error(
            "BHCP2003",
            "retention children must expose record inputs",
            source_name,
            &branch.at,
        ));
    };
    if branch.arguments.len() != child_fields.len() {
        return Err(error(
            "BHCP2003",
            "retention child arguments must exactly cover its typed input fields",
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
                format!(
                    "retention argument does not name child input {:?}",
                    field.name
                ),
                source_name,
                &branch.at,
            ));
        };
        if argument.source == "<expression>" {
            return Err(error(
                "BHCP2003",
                "retention coordinates must be exact parent fields or predecessor outputs",
                source_name,
                &argument.at,
            ));
        }
        let (source_type, symbol, predecessor_output) = if let Some(predecessor) = predecessors
            .iter()
            .find(|predecessor| predecessor.tag == argument.source)
        {
            (
                predecessor.output.clone(),
                "bhcp/kernel.observed-output@0",
                true,
            )
        } else if let Some(parent_type) = parent_field_type(parent_input, &argument.source) {
            (parent_type.clone(), "bhcp/kernel.parent-field@0", false)
        } else {
            return Err(error(
                "BHCP2001",
                format!(
                    "retention argument source {:?} is not a parent field or prior child",
                    argument.source
                ),
                source_name,
                &argument.at,
            ));
        };
        if predecessor_output && bhcp_type_contains_handle(&source_type) {
            return Err(error(
                "BHCP4401",
                "retention predecessor outputs containing resource handles are not executable in this slice",
                source_name,
                &argument.at,
            ));
        }
        if !data_edge_type_compatible(
            &source_type,
            &field.value_type,
            kernel_argument_mode(argument.mode),
        ) {
            return Err(error(
                "BHCP2003",
                "retention data edge does not match the child input type",
                source_name,
                &argument.at,
            ));
        }
        let coordinate = Expression {
            id: ids.next("expr"),
            value_type: BhcpType::Primitive("Text"),
            form: ExpressionForm::Literal(Value::Text(argument.source.clone())),
        };
        lowered.push(KernelArgument {
            name: argument.name.clone(),
            mode: kernel_argument_mode(argument.mode),
            value: Expression {
                id: ids.next("expr"),
                value_type: field.value_type.clone(),
                form: ExpressionForm::Call(symbol.to_owned(), vec![coordinate]),
            },
        });
    }
    lowered.sort_by(|left, right| left.name.cmp(&right.name));
    Ok(lowered)
}

fn check_recursion_bound(
    branch: &crate::parser::SurfaceBranch,
    arguments: &[KernelArgument],
    clauses: &[crate::parser::SurfaceClause],
    guard: Option<&SurfaceExpression>,
    source_name: &str,
) -> Result<RecursionBound> {
    let mut static_bounds = Vec::new();
    for clause in clauses {
        let SurfaceClauseKind::Contract {
            kind: "limit",
            condition:
                SurfaceExpression::Binary {
                    operator,
                    left,
                    right,
                    ..
                },
            ..
        } = &clause.kind
        else {
            continue;
        };
        let (
            "<=",
            SurfaceExpression::Reference { name, .. },
            SurfaceExpression::Literal {
                value: SurfaceLiteral::Integer(maximum),
                ..
            },
        ) = (operator.as_str(), left.as_ref(), right.as_ref())
        else {
            continue;
        };
        if *maximum > 0
            && branch.arguments.iter().any(|argument| {
                argument.name == *name && surface_expression_references(&argument.value, name)
            })
        {
            static_bounds.push(*maximum as u64);
        }
    }
    if let Some(maximum) = static_bounds.into_iter().min() {
        return Ok(RecursionBound::Bounded { maximum });
    }

    let mut measures = branch
        .arguments
        .iter()
        .filter_map(|argument| {
            let SurfaceExpression::Binary {
                operator,
                left,
                right,
                ..
            } = &argument.value
            else {
                return None;
            };
            let (
                "-",
                SurfaceExpression::Reference { name, .. },
                SurfaceExpression::Literal {
                    value: SurfaceLiteral::Integer(step),
                    ..
                },
            ) = (operator.as_str(), left.as_ref(), right.as_ref())
            else {
                return None;
            };
            (*step > 0
                && argument.name == *name
                && checked_measure_lower_bound(clauses, guard, name)
                    .is_some_and(|bound| bound >= *step))
            .then_some((argument.name.as_str(), name.as_str()))
        })
        .collect::<Vec<_>>();
    measures.sort_unstable();
    if let Some((argument_name, _)) = measures.first()
        && let Some(argument) = arguments
            .iter()
            .find(|argument| argument.name == *argument_name)
    {
        return Ok(RecursionBound::WellFounded {
            measure: argument.value.clone(),
        });
    }
    Err(error(
        "BHCP2301",
        "recursive child requires a static bound or a checked decreasing measure",
        source_name,
        &branch.at,
    ))
}

fn surface_expression_references(expression: &SurfaceExpression, name: &str) -> bool {
    match expression {
        SurfaceExpression::Reference {
            name: candidate, ..
        } => candidate == name,
        SurfaceExpression::Unary { operand, .. } => surface_expression_references(operand, name),
        SurfaceExpression::Binary { left, right, .. } => {
            surface_expression_references(left, name) || surface_expression_references(right, name)
        }
        SurfaceExpression::If {
            condition,
            consequent,
            alternative,
            ..
        } => {
            surface_expression_references(condition, name)
                || surface_expression_references(consequent, name)
                || surface_expression_references(alternative, name)
        }
        SurfaceExpression::Call { arguments, .. } => arguments
            .iter()
            .any(|argument| surface_expression_references(argument, name)),
        SurfaceExpression::Literal { .. } => false,
    }
}

fn checked_measure_lower_bound(
    clauses: &[crate::parser::SurfaceClause],
    guard: Option<&SurfaceExpression>,
    name: &str,
) -> Option<i64> {
    clauses
        .iter()
        .filter_map(|clause| {
            let SurfaceClauseKind::Contract {
                kind: "requires",
                condition,
                ..
            } = &clause.kind
            else {
                return None;
            };
            surface_lower_bound(condition, name)
        })
        .chain(guard.and_then(|condition| surface_lower_bound(condition, name)))
        .max()
}

fn surface_lower_bound(expression: &SurfaceExpression, name: &str) -> Option<i64> {
    let SurfaceExpression::Binary {
        operator,
        left,
        right,
        ..
    } = expression
    else {
        return None;
    };
    match (operator.as_str(), left.as_ref(), right.as_ref()) {
        (
            ">=" | ">",
            SurfaceExpression::Reference {
                name: candidate, ..
            },
            SurfaceExpression::Literal {
                value: SurfaceLiteral::Integer(bound),
                ..
            },
        ) if candidate == name => {
            if operator == ">" {
                bound.checked_add(1)
            } else {
                Some(*bound)
            }
        }
        (
            "<=" | "<",
            SurfaceExpression::Literal {
                value: SurfaceLiteral::Integer(bound),
                ..
            },
            SurfaceExpression::Reference {
                name: candidate, ..
            },
        ) if candidate == name => {
            if operator == "<" {
                bound.checked_add(1)
            } else {
                Some(*bound)
            }
        }
        ("&&", _, _) => match (
            surface_lower_bound(left, name),
            surface_lower_bound(right, name),
        ) {
            (Some(left), Some(right)) => Some(left.max(right)),
            (bound @ Some(_), None) | (None, bound @ Some(_)) => bound,
            (None, None) => None,
        },
        ("||", _, _) => {
            Some(surface_lower_bound(left, name)?.min(surface_lower_bound(right, name)?))
        }
        _ => None,
    }
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
        return Err(error(
            "BHCP2003",
            "gate parents and children must expose record inputs",
            source_name,
            &branch.at,
        ));
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
        let mode = kernel_argument_mode(argument.mode);
        let value = if argument.source == "<expression>" {
            let value = lower_gate_condition(&argument.value, parent_input, source_name, ids)?;
            if !data_edge_type_compatible(&value.value_type, &field.value_type, mode) {
                return Err(error(
                    "BHCP2003",
                    "gate expression does not match the child input type",
                    source_name,
                    &argument.at,
                ));
            }
            value
        } else {
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
            if !data_edge_type_compatible(&parent_field.value_type, &field.value_type, mode) {
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
            Expression {
                id: ids.next("expr"),
                value_type: field.value_type.clone(),
                form: ExpressionForm::Call(
                    "bhcp/kernel.parent-field@0".to_owned(),
                    vec![field_name],
                ),
            }
        };
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
            let Some(value_type) = closed_unary_result_type(operator, &operand.value_type) else {
                return Err(error(
                    "BHCP2003",
                    "gate unary condition must preserve Bool",
                    source_name,
                    at,
                ));
            };
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
            let left = lower_gate_condition(left, parent, source_name, ids)?;
            let right = lower_gate_condition(right, parent, source_name, ids)?;
            let value_type =
                closed_binary_result_type(operator, &left.value_type, &right.value_type);
            let Some(value_type) = value_type else {
                return Err(error(
                    "BHCP2003",
                    "gate binary condition is not total and consistently typed",
                    source_name,
                    at,
                ));
            };
            (
                value_type,
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
    type_relations: &TypeRelations,
    ids: &mut Ids,
) -> Result<FunctionDefinition> {
    let ReducerSpecialization {
        input,
        observations,
        output,
        gate_condition,
    } = specialization;
    if !reducer_signature_matches(source, &input, &observations, &output, type_relations) {
        return Err(Diagnostic::plain(
            "BHCP3001",
            "reducer signature does not match its specialization",
        ));
    }
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

fn reducer_signature_matches(
    source: &SurfaceFunction,
    input: &BhcpType,
    observations: &BhcpType,
    output: &BhcpType,
    type_relations: &TypeRelations,
) -> bool {
    let [parent, observed] = source.parameters.as_slice() else {
        return false;
    };
    let SurfaceType::Reduction(result) = &source.result else {
        return false;
    };
    let parameters = source
        .type_parameters
        .iter()
        .cloned()
        .collect::<HashSet<_>>();
    let mut substitutions = HashMap::new();
    let matches =
        surface_type_matches_specialization(
            &parent.value_type,
            input,
            &parameters,
            &mut substitutions,
        ) && surface_type_matches_specialization(
            &observed.value_type,
            observations,
            &parameters,
            &mut substitutions,
        ) && surface_type_matches_specialization(result, output, &parameters, &mut substitutions);
    if !matches || substitutions.len() != source.type_parameters.len() {
        return false;
    }
    let arguments = source
        .type_parameters
        .iter()
        .map(|parameter| {
            substitutions
                .get(parameter)
                .and_then(|argument| CheckedType::from_value(&argument.to_value()).ok())
        })
        .collect::<Option<Vec<_>>>();
    let Some(arguments) = arguments else {
        return false;
    };
    source
        .type_parameter_bounds
        .iter()
        .enumerate()
        .all(|(index, bound)| {
            let Some(bound) = bound else {
                return true;
            };
            let Ok(bound) = surface_type(bound, &source.type_parameters)
                .and_then(|bound| substitute_checked_type(&bound, &arguments))
            else {
                return false;
            };
            is_dynamic_checked_type(&bound)
                || arguments[index].is_subtype_of(&bound, type_relations)
        })
}

fn is_dynamic_checked_type(value: &CheckedType) -> bool {
    matches!(
        value.to_value(),
        Value::Array(parts)
            if matches!(parts.as_slice(), [Value::Text(tag), Value::Text(name)] if tag == "special" && name == "Dynamic")
    )
}

fn surface_type_matches_specialization(
    declared: &SurfaceType,
    actual: &BhcpType,
    parameters: &HashSet<String>,
    substitutions: &mut HashMap<String, BhcpType>,
) -> bool {
    match (declared, actual) {
        (SurfaceType::Parameter(name), actual) if parameters.contains(name) => {
            match substitutions.get(name) {
                Some(previous) => previous == actual,
                None => {
                    substitutions.insert(name.clone(), actual.clone());
                    true
                }
            }
        }
        (SurfaceType::Primitive(left), BhcpType::Primitive(right)) => left == right,
        (SurfaceType::Exact(left), BhcpType::ExactNumber(right)) => left == right,
        (SurfaceType::Record(left), BhcpType::Record(right)) if left.len() == right.len() => {
            left.iter().all(|field| {
                right
                    .iter()
                    .find(|candidate| candidate.name == field.name)
                    .is_some_and(|candidate| {
                        surface_type_matches_specialization(
                            &field.value_type,
                            &candidate.value_type,
                            parameters,
                            substitutions,
                        )
                    })
            })
        }
        (
            SurfaceType::Nominal {
                symbol: left_symbol,
                arguments: left_arguments,
            },
            BhcpType::Nominal(right_symbol, right_arguments),
        ) if left_symbol == right_symbol && left_arguments.len() == right_arguments.len() => {
            left_arguments
                .iter()
                .zip(right_arguments)
                .all(|(left, right)| {
                    surface_type_matches_specialization(left, right, parameters, substitutions)
                })
        }
        (SurfaceType::Reduction(left), BhcpType::Reduction(right))
        | (SurfaceType::List(left), BhcpType::List(right))
        | (SurfaceType::Option(left), BhcpType::Option(right)) => {
            surface_type_matches_specialization(left, right, parameters, substitutions)
        }
        (
            SurfaceType::Handle {
                ownership,
                access,
                usage,
                lifetime,
                value_type,
            },
            BhcpType::Handle(actual),
        ) => {
            ownership == &actual.ownership
                && access.as_deref().unwrap_or(if ownership == "shared" {
                    "read"
                } else {
                    "write"
                }) == actual.access
                && usage.as_deref().unwrap_or("unrestricted") == actual.usage
                && lifetime.as_deref().unwrap_or("goal") == actual.lifetime
                && surface_type_matches_specialization(
                    value_type,
                    &actual.value_type,
                    parameters,
                    substitutions,
                )
        }
        _ => false,
    }
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
                "bhcp/kernel.unobserved-unit@0"
                    if argument_types == [observations_type.clone()] =>
                {
                    BhcpType::Primitive("Unit")
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
                if resource_symbol(&binding.value_type).is_none() {
                    return Err(error(
                        "BHCP4501",
                        format!(
                            "effect resource coordinate {name:?} must reference a nominal resource or handle"
                        ),
                        source_name,
                        at,
                    ));
                }
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
