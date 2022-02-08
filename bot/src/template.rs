use std::collections::HashSet;
use std::fmt;
use std::io;
use std::string;

lazy_static::lazy_static! {
    static ref REGISTRY: handlebars::Handlebars<'static> = {
        let mut reg = handlebars::Handlebars::new();
        reg.register_escape_fn(|s| s.to_string());
        reg
    };
}

#[derive(Debug, Clone)]
pub struct Template {
    source: String,
    template: handlebars::template::Template,
}

impl<'de> serde::Deserialize<'de> for Template {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use std::fmt::Write as _;

        let s = TemplateData::deserialize(deserializer)?;

        let s = match s {
            TemplateData::List(list) => {
                let mut s = String::new();

                let mut it = list.iter();

                let back = it.next_back();

                for line in it {
                    writeln!(&mut s, "{}", line).map_err(serde::de::Error::custom)?;
                }

                if let Some(line) = back {
                    write!(&mut s, "{}", line).map_err(serde::de::Error::custom)?;
                }

                s
            }
            TemplateData::String(s) => s,
        };

        let template = handlebars::Template::compile(&s).map_err(serde::de::Error::custom)?;

        return Ok(Template {
            source: s,
            template,
        });

        #[derive(serde::Deserialize)]
        #[serde(untagged)]
        enum TemplateData {
            // a single string.
            String(String),
            // line-separated list.
            List(Vec<String>),
        }
    }
}

impl serde::Serialize for Template {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.source.serialize(serializer)
    }
}

impl Template {
    pub fn compile(s: &str) -> Result<Template, anyhow::Error> {
        Ok(Template {
            source: s.to_string(),
            template: handlebars::Template::compile(s)?,
        })
    }

    /// Render the template to the given output.
    pub fn render(
        &self,
        out: &mut impl io::Write,
        data: impl serde::Serialize,
    ) -> Result<(), anyhow::Error> {
        let mut output = WriteOutput::new(out);
        self.render_internal(&mut output, data)
    }

    /// Render the template to a string.
    pub fn render_to_string(&self, data: impl serde::Serialize) -> Result<String, anyhow::Error> {
        let mut output = StringOutput::new();
        self.render_internal(&mut output, data)?;
        output.into_string().map_err(Into::into)
    }

    /// Test if the template has the given variable.
    pub fn vars(&self) -> HashSet<String> {
        use handlebars::template::{HelperTemplate, Parameter, TemplateElement};
        use std::collections::VecDeque;

        let mut out = HashSet::new();

        for e in &self.template.elements {
            collect_element(&mut out, e);
        }

        return out;

        /// Helper to collect all expressions without recursing.
        fn collect_element<'e>(out: &mut HashSet<String>, e: &'e TemplateElement) {
            let mut queue = VecDeque::new();

            queue.push_back(e);

            while let Some(e) = queue.pop_front() {
                match e {
                    TemplateElement::Expression(helper) => {
                        collect_helper(out, &mut queue, &*helper);
                    }
                    TemplateElement::HtmlExpression(param) => {
                        collect_helper(out, &mut queue, param);
                    }
                    _ => (),
                }
            }
        }

        fn collect_parameter<'e>(
            out: &mut HashSet<String>,
            queue: &mut VecDeque<&'e TemplateElement>,
            p: &'e Parameter,
        ) {
            match p {
                Parameter::Subexpression(e) => {
                    queue.push_back(&*e.element);
                }
                p => {
                    if let Some(name) = p.as_name() {
                        out.insert(name.to_string());
                    }
                }
            }
        }

        fn collect_helper<'e>(
            out: &mut HashSet<String>,
            queue: &mut VecDeque<&'e TemplateElement>,
            e: &'e HelperTemplate,
        ) {
            collect_parameter(out, queue, &e.name);

            for p in &e.params {
                collect_parameter(out, queue, p);
            }
        }
    }

    /// Access the source of the template.
    pub fn source(&self) -> &str {
        self.source.as_str()
    }

    /// Render the template to the given output.
    fn render_internal(
        &self,
        output: &mut dyn handlebars::Output,
        data: impl serde::Serialize,
    ) -> Result<(), anyhow::Error> {
        use handlebars::Renderable as _;

        let ctx = handlebars::Context::wraps(data)?;
        let mut render_context = handlebars::RenderContext::new(None);
        self.template
            .render(&*REGISTRY, &ctx, &mut render_context, output)
            .map_err(Into::into)
    }
}

impl std::str::FromStr for Template {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::compile(s)
    }
}

impl fmt::Display for Template {
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.source.fmt(fmt)
    }
}

pub struct WriteOutput<W> {
    write: W,
}

impl<W> handlebars::Output for WriteOutput<W>
where
    W: io::Write,
{
    fn write(&mut self, seg: &str) -> Result<(), io::Error> {
        self.write.write_all(seg.as_bytes())
    }
}

impl<W> WriteOutput<W> {
    pub fn new(write: W) -> WriteOutput<W> {
        WriteOutput { write }
    }
}

pub struct StringOutput {
    buf: Vec<u8>,
}

impl handlebars::Output for StringOutput {
    fn write(&mut self, seg: &str) -> Result<(), io::Error> {
        self.buf.extend_from_slice(seg.as_bytes());
        Ok(())
    }
}

impl StringOutput {
    pub fn new() -> StringOutput {
        StringOutput {
            buf: Vec::with_capacity(1024),
        }
    }

    pub fn into_string(self) -> Result<String, string::FromUtf8Error> {
        String::from_utf8(self.buf)
    }
}

impl Default for StringOutput {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::Template;
    use anyhow::Error;
    use std::collections::HashSet;

    #[test]
    pub fn test_template_vars() -> Result<(), Error> {
        assert_eq!(
            vec!["foo", "bar", "baz"]
                .into_iter()
                .map(|s| s.to_string())
                .collect::<HashSet<String>>(),
            Template::compile("{{foo}} {{bar}} is the {{baz}}")?.vars()
        );

        Ok(())
    }
}
