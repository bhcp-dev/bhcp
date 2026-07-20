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

    pub fn decide(&self, tenant: &str, subject: &str, action: &str, resource: &str) -> Decision {
        let selected = self
            .rules
            .iter()
            .filter(|rule| {
                rule.enabled
                    && rule.tenant == tenant
                    && pattern_matches(&rule.subject, subject)
                    && pattern_matches(&rule.action, action)
                    && pattern_matches(&rule.resource, resource)
            })
            .max_by(compare_rules);

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

fn compare_rules(a: &&Rule, b: &&Rule) -> std::cmp::Ordering {
    rule_rank(a).cmp(&rule_rank(b))
}

fn rule_rank(rule: &Rule) -> (u8, i32, u8, std::cmp::Reverse<&str>) {
    let specificity = [
        rule.subject.as_str() != "*",
        rule.action.as_str() != "*",
        rule.resource.as_str() != "*",
    ]
    .into_iter()
    .map(u8::from)
    .sum();
    let effect_rank = match rule.effect {
        Effect::Allow => 0,
        Effect::Deny => 1,
    };

    (
        specificity,
        rule.priority,
        effect_rank,
        std::cmp::Reverse(rule.id.as_str()),
    )
}
