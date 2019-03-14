use crate::utils;

/// Command aliases.
#[derive(Debug, Default, serde::Deserialize)]
#[serde(transparent)]
pub struct Aliases {
    aliases: Vec<MatchReplace>,
}

impl Aliases {
    pub fn lookup<'a>(&self, it: utils::Words<'a>) -> Option<String> {
        let it = it.into_iter();

        for alias in &self.aliases {
            if let Some(out) = alias.matches(it.clone()) {
                return Some(out);
            }
        }

        None
    }
}

#[derive(Debug, serde::Deserialize)]
struct MatchReplace {
    #[serde(rename = "match")]
    m: Match,
    replace: Replace,
}

impl MatchReplace {
    /// Test if the given input matches and return the corresonding replacement if it does.
    pub fn matches<'a>(&self, mut it: utils::Words<'a>) -> Option<String> {
        match self.m {
            Match::Command(ref name) => match it.next() {
                Some(value) if value.starts_with('!') => {
                    if name == &value[1..] {
                        return self.replace.render(it);
                    }
                }
                _ => {}
            },
        }

        None
    }
}

/// Thing to match against.
#[derive(Debug)]
enum Match {
    Command(String),
}

impl<'de> serde::Deserialize<'de> for Match {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;

        if s.starts_with("!") {
            return Ok(Match::Command(s[1..].to_string()));
        }

        Err(serde::de::Error::custom("not a valid match"))
    }
}

/// Replacement.
#[derive(Debug)]
enum Replace {
    Template(mustache::Template),
}

impl Replace {
    pub fn render(&self, it: utils::Words<'_>) -> Option<String> {
        return match *self {
            Replace::Template(ref template) => {
                let data = Data { rest: it.rest() };

                match template.render_to_string(&data) {
                    Ok(s) => Some(s),
                    Err(e) => {
                        log::error!("failed to render alias: {}", e);
                        None
                    }
                }
            }
        };

        #[derive(serde::Serialize)]
        struct Data<'a> {
            rest: &'a str,
        }
    }
}

impl<'de> serde::Deserialize<'de> for Replace {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        let t = mustache::compile_str(&s).map_err(serde::de::Error::custom)?;
        Ok(Replace::Template(t))
    }
}
