use globset::{Glob, GlobSet, GlobSetBuilder};

pub struct DisplayFilter {
    matchers: GlobSet,
    negated: bool,
    pass_all: bool,
}

impl DisplayFilter {
    pub fn parse(pattern: &str) -> Result<Self, String> {
        let trimmed = pattern.trim();
        if trimmed == "*" || trimmed.is_empty() {
            return Ok(Self {
                matchers: GlobSetBuilder::new().build().map_err(|e| e.to_string())?,
                negated: false,
                pass_all: true,
            });
        }

        let (pat, negated) = if let Some(inner) = trimmed
            .strip_prefix("^(")
            .and_then(|s| s.strip_suffix(')'))
        {
            (inner, true)
        } else {
            (trimmed, false)
        };

        let mut builder = GlobSetBuilder::new();
        for token in pat.split(';').filter(|s| !s.trim().is_empty()) {
            let glob = Glob::new(token.trim()).map_err(|e| e.to_string())?;
            builder.add(glob);
        }
        Ok(Self {
            matchers: builder.build().map_err(|e| e.to_string())?,
            negated,
            pass_all: false,
        })
    }

    pub fn matches(&self, name: &str) -> bool {
        if self.pass_all {
            return true;
        }
        let hit = self.matchers.is_match(name);
        if self.negated {
            !hit
        } else {
            hit
        }
    }
}
