use fluent_syntax::ast::{self, Entry, Expression, InlineExpression, PatternElement};

#[derive(Debug)]
pub struct EntryPlaceable(pub &'static str);

impl EntryPlaceable {
    pub fn placeable_from_expression(expression: &Expression<&'static str>) -> Option<Self> {
        match expression {
            ast::Expression::Select { .. } => {
                // Select expressions are not supported yet
                None
            }
            ast::Expression::Inline(inline) => {
                if let InlineExpression::VariableReference { id } = inline {
                    Some(Self(id.name))
                } else {
                    // Only `fluent_syntax::ast::InlineExpression::VariableReference` supported
                    None
                }
            }
        }
    }
}

#[derive(Debug)]
pub struct TranslationEntry {
    pub documentation: String,
    pub key: &'static str,
    pub placeables: Vec<EntryPlaceable>,
}

#[derive(Debug, PartialEq, Eq, thiserror::Error)]
pub enum LocalizationFileContentsParsingError {
    #[error("Localization file contents couldn't be parsed")]
    Generic,
}

pub fn parse(
    input: &'static str,
) -> Result<impl Iterator<Item = TranslationEntry>, LocalizationFileContentsParsingError> {
    let resource = fluent_syntax::parser::parse(input)
        .map_err(|_| LocalizationFileContentsParsingError::Generic)?;
    Ok(resource.body.into_iter().flat_map(|e| {
        if let Entry::Message(message) = e {
            let mut documentation = String::new();
            let key = message.id.name;
            let placeables = if let Some(pattern) = message.value {
                pattern
                    .elements
                    .into_iter()
                    .filter_map(|pattern_element| match pattern_element {
                        PatternElement::TextElement { value } => {
                            documentation.push_str(value);
                            None
                        }
                        PatternElement::Placeable { expression } => {
                            if let Some(placeable_element) =
                                EntryPlaceable::placeable_from_expression(&expression)
                            {
                                documentation
                                    .push_str(format!("{{${}}}", placeable_element.0).as_str());
                                Some(placeable_element)
                            } else {
                                None
                            }
                        }
                    })
                    .collect()
            } else {
                vec![]
            };
            Some(TranslationEntry {
                documentation,
                key,
                placeables,
            })
        } else {
            None
        }
    }))
}
