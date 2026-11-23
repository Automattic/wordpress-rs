use fluent_syntax::{
    ast::{Entry, Expression, InlineExpression, PatternElement},
    parser::Slice,
};

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EntryPlaceable(pub String);

impl EntryPlaceable {
    /// Returns the variable an inline expression reads, when it's a variable.
    fn from_inline<'s, S>(inline: &InlineExpression<S>) -> Option<Self>
    where
        S: Slice<'s>,
    {
        if let InlineExpression::VariableReference { id } = inline {
            Some(Self(id.name.as_ref().to_string()))
        } else {
            // Only `fluent_syntax::ast::InlineExpression::VariableReference` supported
            None
        }
    }
}

/// Adds the variables a pattern reads to `placeables`, each once, and its text to
/// `documentation`. A select expression, such as a plural, reads its selector and every
/// variable in its variants; its documentation is its default variant.
fn read_pattern<'s, S>(
    elements: &[PatternElement<S>],
    placeables: &mut Vec<EntryPlaceable>,
    documentation: &mut String,
) where
    S: Slice<'s>,
{
    for element in elements {
        match element {
            PatternElement::TextElement { value } => documentation.push_str(value.as_ref()),
            PatternElement::Placeable {
                expression: Expression::Inline(inline),
            } => {
                if let Some(placeable) = EntryPlaceable::from_inline(inline) {
                    documentation.push_str(format!("{{${}}}", placeable.0).as_str());
                    add(placeable, placeables);
                }
            }
            PatternElement::Placeable {
                expression: Expression::Select { selector, variants },
            } => {
                if let Some(placeable) = EntryPlaceable::from_inline(selector) {
                    add(placeable, placeables);
                }
                for variant in variants {
                    let mut variant_documentation = String::new();
                    let mut variant_placeables = Vec::new();
                    read_pattern(
                        &variant.value.elements,
                        &mut variant_placeables,
                        &mut variant_documentation,
                    );
                    for placeable in variant_placeables {
                        add(placeable, placeables);
                    }
                    if variant.default {
                        documentation.push_str(&variant_documentation);
                    }
                }
            }
        }
    }
}

fn add(placeable: EntryPlaceable, placeables: &mut Vec<EntryPlaceable>) {
    if !placeables.contains(&placeable) {
        placeables.push(placeable);
    }
}

#[derive(Debug)]
pub struct TranslationEntry {
    pub documentation: String,
    pub key: String,
    pub placeables: Vec<EntryPlaceable>,
}

#[derive(Debug, PartialEq, Eq, thiserror::Error)]
pub enum LocalizationFileContentsParsingError {
    #[error("Localization file contents couldn't be parsed")]
    Generic,
}

pub fn parse<'s, S>(
    input: S,
) -> Result<impl Iterator<Item = TranslationEntry>, LocalizationFileContentsParsingError>
where
    S: Slice<'s>,
{
    let resource = fluent_syntax::parser::parse(input)
        .map_err(|_| LocalizationFileContentsParsingError::Generic)?;
    Ok(resource.body.into_iter().flat_map(|e| {
        if let Entry::Message(message) = e {
            let mut documentation = String::new();
            let mut placeables = Vec::new();
            if let Some(pattern) = message.value {
                read_pattern(&pattern.elements, &mut placeables, &mut documentation);
            }
            Some(TranslationEntry {
                documentation,
                key: message.id.name.as_ref().to_string(),
                placeables,
            })
        } else {
            None
        }
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(source: &str) -> TranslationEntry {
        parse(source)
            .expect("the source parses")
            .next()
            .expect("the source has a message")
    }

    #[test]
    fn reads_variables() {
        let entry = entry("greeting = Hello, {$name}, from {$place}.\n");
        assert_eq!(entry.key, "greeting");
        assert_eq!(
            entry.placeables,
            [
                EntryPlaceable("name".to_string()),
                EntryPlaceable("place".to_string())
            ]
        );
        assert_eq!(entry.documentation, "Hello, {$name}, from {$place}.");
    }

    #[test]
    fn reads_a_plural_selector_and_its_variants_variables_once() {
        let entry = entry(
            "likes = { $count ->\n    [one] { $count } like by {$name}\n   *[other] { $count } likes by {$name}\n}\n",
        );
        assert_eq!(
            entry.placeables,
            [
                EntryPlaceable("count".to_string()),
                EntryPlaceable("name".to_string())
            ]
        );
        assert_eq!(entry.documentation, "{$count} likes by {$name}");
    }
}
