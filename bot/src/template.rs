use hashbrown::HashSet;
use std::{fmt, io, string};

lazy_static::lazy_static! {
    static ref REGISTRY: handlebars::Handlebars = {
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
            source: s.to_string(),
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

impl Template {
    pub fn compile(s: impl AsRef<str>) -> Result<Template, failure::Error> {
        let source = s.as_ref();

        Ok(Template {
            source: source.to_string(),
            template: handlebars::Template::compile(source)?,
        })
    }

    /// Render the template to the given output.
    pub fn render(
        &self,
        out: &mut impl io::Write,
        data: impl serde::Serialize,
    ) -> Result<(), failure::Error> {
        let mut output = WriteOutput::new(out);
        self.render_internal(&mut output, data)
    }

    /// Render the template to a string.
    pub fn render_to_string(&self, data: impl serde::Serialize) -> Result<String, failure::Error> {
        let mut output = StringOutput::new();
        self.render_internal(&mut output, data)?;
        output.into_string().map_err(Into::into)
    }

    /// Test if the template has the given variable.
    pub fn vars(&self) -> HashSet<String> {
        use handlebars::template::{Parameter, TemplateElement};

        let mut out = HashSet::new();

        for e in &self.template.elements {
            collect_vars(&mut out, e);
        }

        return out;

        fn collect_vars(out: &mut HashSet<String>, e: &TemplateElement) {
            match e {
                TemplateElement::Expression(ref p) => match *p {
                    Parameter::Name(ref name) => {
                        out.insert(name.to_string());
                    }
                    Parameter::Literal(_) => (),
                    Parameter::Subexpression(ref e) => collect_vars(out, &e.element),
                },
                _ => (),
            }
        }
    }

    /// Render the template to the given output.
    fn render_internal(
        &self,
        output: &mut handlebars::Output,
        data: impl serde::Serialize,
    ) -> Result<(), failure::Error> {
        use handlebars::Renderable as _;

        let ctx = handlebars::Context::wraps(data)?;
        let mut render_context = handlebars::RenderContext::new(None);
        self.template
            .render(&*REGISTRY, &ctx, &mut render_context, output)
            .map_err(Into::into)
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
