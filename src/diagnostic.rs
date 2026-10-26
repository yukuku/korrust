#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticKind {
    Unclassified,
    UnsupportedLanguage,
    InvalidKotlin,
    MissingDependency,
    Setup,
    Io,
}

impl DiagnosticKind {
    pub fn category(self) -> &'static str {
        match self {
            Self::Unclassified => "unclassified",
            Self::UnsupportedLanguage => "unsupported_language",
            Self::InvalidKotlin => "invalid_kotlin",
            Self::MissingDependency => "missing_dependency",
            Self::Setup => "setup",
            Self::Io => "io",
        }
    }
}

#[derive(Debug, Clone)]
pub struct Diagnostic {
    pub offset: usize,
    pub message: String,
    pub source_identity: Option<String>,
    pub kind: DiagnosticKind,
    pub code: &'static str,
    pub(crate) type_name: Option<Box<str>>,
}

impl Diagnostic {
    pub fn new(offset: usize, message: impl Into<String>) -> Self {
        Self {
            offset,
            message: message.into(),
            source_identity: None,
            kind: DiagnosticKind::Unclassified,
            code: "UNCLASSIFIED",
            type_name: None,
        }
    }

    pub fn with_kind(mut self, kind: DiagnosticKind, code: &'static str) -> Self {
        self.kind = kind;
        self.code = code;
        self
    }

    pub(crate) fn for_type(mut self, name: &str) -> Self {
        self.type_name = Some(name.into());
        self
    }

    pub fn for_source(mut self, identity: &str) -> Self {
        self.source_identity = Some(identity.into());
        self
    }

    fn location(&self, source: &str) -> (usize, usize) {
        let mut end = self.offset.min(source.len());
        while !source.is_char_boundary(end) {
            end -= 1;
        }
        let prefix = &source[..end];
        let line = prefix.bytes().filter(|byte| *byte == b'\n').count() + 1;
        let column = prefix.rsplit('\n').next().unwrap_or("").chars().count() + 1;
        (line, column)
    }

    pub fn render(&self, path: &str, source: &str) -> String {
        let (line, column) = self.location(source);
        format!("{path}:{line}:{column}: error: {}", self.message)
    }

    pub fn render_json(&self, path: Option<&str>, source: Option<&str>) -> String {
        let identity = self
            .source_identity
            .as_deref()
            .map(json_string)
            .unwrap_or_else(|| "null".into());
        let mut result = format!(
            "{{\"source_identity\":{identity},\"offset\":{},\"category\":{},\"code\":{},\"message\":{}",
            self.offset,
            json_string(self.kind.category()),
            json_string(self.code),
            json_string(&self.message),
        );
        if let Some(path) = path {
            result.push_str(&format!(",\"path\":{}", json_string(path)));
        }
        if let Some(source) = source {
            let (line, column) = self.location(source);
            result.push_str(&format!(",\"line\":{line},\"column\":{column}"));
        }
        result.push('}');
        result
    }
}

fn json_string(value: &str) -> String {
    use std::fmt::Write;
    let mut result = String::from("\"");
    for character in value.chars() {
        match character {
            '"' => result.push_str("\\\""),
            '\\' => result.push_str("\\\\"),
            '\n' => result.push_str("\\n"),
            '\r' => result.push_str("\\r"),
            '\t' => result.push_str("\\t"),
            '\u{08}' => result.push_str("\\b"),
            '\u{0c}' => result.push_str("\\f"),
            character if character < '\u{20}' => {
                write!(result, "\\u{:04x}", character as u32).unwrap();
            }
            character => result.push(character),
        }
    }
    result.push('"');
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unclassified_errors_do_not_guess_categories_from_messages() {
        let diagnostic = Diagnostic::new(0, "Unsupported missing class file");
        assert_eq!(diagnostic.kind, DiagnosticKind::Unclassified);
        assert_eq!(diagnostic.code, "UNCLASSIFIED");
    }

    #[test]
    fn json_escapes_paths_messages_and_identities() {
        let diagnostic = Diagnostic::new(4, "\"quoted\"\\\n\r\t\u{08}\u{0c}\0\u{1f}λ")
            .for_source("source\n\".kt")
            .with_kind(DiagnosticKind::UnsupportedLanguage, "TEST_UNSUPPORTED");
        assert_eq!(
            diagnostic.render_json(Some("a\\b.kt"), Some("λ\nx")),
            "{\"source_identity\":\"source\\n\\\".kt\",\"offset\":4,\"category\":\"unsupported_language\",\"code\":\"TEST_UNSUPPORTED\",\"message\":\"\\\"quoted\\\"\\\\\\n\\r\\t\\b\\f\\u0000\\u001fλ\",\"path\":\"a\\\\b.kt\",\"line\":2,\"column\":2}"
        );
        assert_eq!(
            Diagnostic::new(0, "error").render_json(None, None),
            "{\"source_identity\":null,\"offset\":0,\"category\":\"unclassified\",\"code\":\"UNCLASSIFIED\",\"message\":\"error\"}"
        );
    }

    #[test]
    fn rendering_clamps_offsets_to_utf8_boundaries() {
        assert_eq!(
            Diagnostic::new(1, "error").render("a.kt", "λ"),
            "a.kt:1:1: error: error"
        );
    }
}
