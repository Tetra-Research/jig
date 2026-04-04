use std::fmt;

#[derive(Debug, Clone, serde::Serialize)]
pub struct StructuredError {
    pub what: String,
    #[serde(rename = "where")]
    pub where_: String,
    pub why: String,
    pub hint: String,
}

impl fmt::Display for StructuredError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "error: {}\n  where: {}\n  why: {}\n  hint: {}",
            self.what, self.where_, self.why, self.hint
        )
    }
}

#[derive(Debug, thiserror::Error)]
#[allow(dead_code)] // Variants used incrementally across phases
pub enum JigError {
    #[error("{0}")]
    RecipeValidation(StructuredError),
    #[error("{0}")]
    TemplateRendering(StructuredError),
    #[error("{0}")]
    FileOperation(StructuredError),
    #[error("{}", format_variable_errors(.0))]
    VariableValidation(Vec<StructuredError>),
}

fn format_variable_errors(errors: &[StructuredError]) -> String {
    errors
        .iter()
        .map(|e| e.to_string())
        .collect::<Vec<_>>()
        .join("\n")
}

impl JigError {
    pub fn exit_code(&self) -> i32 {
        match self {
            JigError::RecipeValidation(_) => 1,
            JigError::TemplateRendering(_) => 2,
            JigError::FileOperation(_) => 3,
            JigError::VariableValidation(_) => 4,
        }
    }

    #[allow(dead_code)] // Used in tests and later phases
    pub fn structured_error(&self) -> &StructuredError {
        match self {
            JigError::RecipeValidation(e)
            | JigError::TemplateRendering(e)
            | JigError::FileOperation(e) => e,
            JigError::VariableValidation(errors) => &errors[0],
        }
    }

    #[allow(dead_code)]
    pub fn structured_errors(&self) -> &[StructuredError] {
        match self {
            JigError::RecipeValidation(e)
            | JigError::TemplateRendering(e)
            | JigError::FileOperation(e) => std::slice::from_ref(e),
            JigError::VariableValidation(errors) => errors,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exit_code_mapping() {
        let err = StructuredError {
            what: "test".into(),
            where_: "test".into(),
            why: "test".into(),
            hint: "test".into(),
        };
        assert_eq!(JigError::RecipeValidation(err.clone()).exit_code(), 1);
        assert_eq!(JigError::TemplateRendering(err.clone()).exit_code(), 2);
        assert_eq!(JigError::FileOperation(err.clone()).exit_code(), 3);
        assert_eq!(JigError::VariableValidation(vec![err]).exit_code(), 4);
    }

    #[test]
    fn structured_error_has_all_fields() {
        let err = StructuredError {
            what: "file not found".into(),
            where_: "recipe.yaml".into(),
            why: "the file does not exist at the specified path".into(),
            hint: "check the file path and try again".into(),
        };
        let display = format!("{err}");
        assert!(display.contains("file not found"));
        assert!(display.contains("recipe.yaml"));
        assert!(display.contains("does not exist"));
        assert!(display.contains("check the file path"));
    }

    #[test]
    fn structured_error_serializes_where_field() {
        let err = StructuredError {
            what: "w".into(),
            where_: "x".into(),
            why: "y".into(),
            hint: "z".into(),
        };
        let json = serde_json::to_value(&err).unwrap();
        assert_eq!(json["where"], "x");
        assert!(json.get("where_").is_none());
    }

    #[test]
    fn multiple_variable_errors_display() {
        let errors = vec![
            StructuredError {
                what: "missing required variable".into(),
                where_: "variable 'name'".into(),
                why: "no value provided".into(),
                hint: "add 'name' to --vars".into(),
            },
            StructuredError {
                what: "type mismatch".into(),
                where_: "variable 'count'".into(),
                why: "expected number, got string".into(),
                hint: "provide a numeric value".into(),
            },
        ];
        let err = JigError::VariableValidation(errors);
        let display = format!("{err}");
        assert!(display.contains("missing required variable"));
        assert!(display.contains("type mismatch"));
        assert_eq!(err.structured_errors().len(), 2);
    }
}
