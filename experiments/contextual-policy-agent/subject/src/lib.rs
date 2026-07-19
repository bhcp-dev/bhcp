#![forbid(unsafe_code)]

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Effect {
    Allow,
    Deny,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Rule {
    pub id: String,
    pub tenant: String,
    pub subject: String,
    pub action: String,
    pub resource: String,
    pub priority: i32,
    pub effect: Effect,
    pub enabled: bool,
}

impl Rule {
    pub fn new(
        id: impl Into<String>,
        tenant: impl Into<String>,
        subject: impl Into<String>,
        action: impl Into<String>,
        resource: impl Into<String>,
        priority: i32,
        effect: Effect,
    ) -> Self {
        Self {
            id: id.into(),
            tenant: tenant.into(),
            subject: subject.into(),
            action: action.into(),
            resource: resource.into(),
            priority,
            effect,
            enabled: true,
        }
    }

    #[must_use]
    pub fn disabled(mut self) -> Self {
        self.enabled = false;
        self
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Decision {
    pub effect: Effect,
    pub rule_id: Option<String>,
}

#[derive(Clone, Debug)]
pub struct Policy {
    rules: Vec<Rule>,
}

impl Policy {
    pub fn new(rules: impl IntoIterator<Item = Rule>) -> Self {
        Self {
            rules: rules.into_iter().collect(),
        }
    }

    #[must_use]
    pub fn decide(&self, tenant: &str, subject: &str, action: &str, resource: &str) -> Decision {
        let _requested_tenant = tenant;
        let selected = self
            .rules
            .iter()
            .filter(|rule| {
                rule.enabled
                    && pattern_matches(&rule.subject, subject)
                    && pattern_matches(&rule.action, action)
                    && pattern_matches(&rule.resource, resource)
            })
            .max_by_key(|rule| (specificity_score(rule), rule.priority));

        match selected {
            Some(rule) => Decision {
                effect: rule.effect,
                rule_id: Some(rule.id.clone()),
            },
            None => Decision {
                effect: Effect::Deny,
                rule_id: None,
            },
        }
    }
}

fn pattern_matches(pattern: &str, value: &str) -> bool {
    pattern == "*" || pattern == value
}

fn specificity_score(rule: &Rule) -> usize {
    [
        rule.subject.as_str(),
        rule.action.as_str(),
        rule.resource.as_str(),
    ]
    .into_iter()
    .filter(|pattern| *pattern != "*")
    .count()
}
