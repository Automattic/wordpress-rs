use serde::{
    Deserialize,
    de::{self, DeserializeOwned, Unexpected},
};
use serde_json::value::RawValue;
use std::{fmt, marker::PhantomData};

/// Deserialize a value that may be represented as an empty array.
///
/// Some APIs return `[]` instead of `null` when a value is absent. This function
/// handles both representations, returning `None` for empty arrays and deserializing
/// the value normally for objects.
///
/// Accepts:
/// - Empty JSON array `[]` → `None`
/// - JSON object `{...}` → `Some(T)`
///
/// # Errors
///
/// Returns an error for non-empty arrays or incompatible types.
pub fn deserialize_empty_vec_as_none<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: de::Deserializer<'de>,
    T: Deserialize<'de>,
{
    deserializer.deserialize_any(DeserializeEmptyVecAsNone::<T>(PhantomData))
}

/// Deserialize a `Vec` that may be represented as `null`.
///
/// Some APIs return `null` instead of `[]` when a collection is empty. This function
/// handles both representations, returning an empty `Vec` for `null`.
///
/// Accepts:
/// - JSON array `[...]` → `Vec<T>` with deserialized elements
/// - `null` → empty `Vec<T>`
///
/// # Errors
///
/// Returns an error for incompatible types like booleans, numbers, or strings.
pub fn deserialize_null_as_empty_vec<'de, D, T>(deserializer: D) -> Result<Vec<T>, D::Error>
where
    D: de::Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::<Vec<T>>::deserialize(deserializer)
        .map(|opt| opt.unwrap_or_default())
        .map_err(|err| {
            serde::de::Error::custom(
                err.to_string()
                    .replace("expected a sequence", "expected null or a sequence"),
            )
        })
}

/// A list read one element at a time, so that an element that can't be read doesn't fail the
/// whole list.
///
/// Reading a `Vec<T>` fails at the first element that isn't a valid `T`, and every other element
/// is lost with it. This reads them all: each valid one goes to [`Self::valid`], and each of the
/// others to [`Self::invalid`] with the reason, so that nothing is dropped without a trace.
///
/// What isn't a list is still an error, and so is JSON that isn't well formed. `null` is one too:
/// read an `Option<LenientVec<T>>` where the list can be `null`.
///
/// Each element's JSON is copied before it's parsed, one element at a time. That takes about a
/// tenth longer than reading a `Vec<T>`.
///
/// # Where it can be used
///
/// Only with JSON, and only where it's read from the JSON itself. Inside a struct that's flattened
/// into another with `#[serde(flatten)]`, or inside an untagged or internally tagged enum, serde
/// reads from a buffered copy that an element's JSON can't be taken from, and reading the list is
/// an error.
#[derive(Debug)]
pub struct LenientVec<T> {
    /// The elements that were read, in the order they arrived.
    pub valid: Vec<T>,
    /// The elements that couldn't be read, in the order they arrived.
    pub invalid: Vec<InvalidElement>,
}

/// An element of a [`LenientVec`] that couldn't be read.
#[derive(Debug)]
pub struct InvalidElement {
    /// The element, as it arrived.
    pub json: Box<RawValue>,
    /// Why it couldn't be read. Its line and column count from the start of the element.
    pub error: serde_json::Error,
}

impl<T> Default for LenientVec<T> {
    fn default() -> Self {
        Self {
            valid: Vec::new(),
            invalid: Vec::new(),
        }
    }
}

impl<'de, T> Deserialize<'de> for LenientVec<T>
where
    T: DeserializeOwned,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        deserializer.deserialize_seq(LenientVecVisitor(PhantomData))
    }
}

struct LenientVecVisitor<T>(PhantomData<T>);

impl<'de, T> de::Visitor<'de> for LenientVecVisitor<T>
where
    T: DeserializeOwned,
{
    type Value = LenientVec<T>;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a sequence")
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: de::SeqAccess<'de>,
    {
        let mut list = LenientVec::default();
        // serde can't carry on after an element fails part of the way through it, so each
        // element's JSON is taken whole first, and parsed on its own.
        while let Some(json) = seq.next_element::<Box<RawValue>>()? {
            match serde_json::from_str::<T>(json.get()) {
                Ok(element) => list.valid.push(element),
                Err(error) => list.invalid.push(InvalidElement { json, error }),
            }
        }
        Ok(list)
    }
}

/// A visitor that deserializes either an empty array or a value of type `T`.
///
/// Use this with a custom fallback function to provide a default value when
/// an empty array is encountered.
pub struct DeserializeEmptyVecOrT<T> {
    fallback: Box<dyn Fn() -> T>,
}

impl<T> DeserializeEmptyVecOrT<T> {
    pub fn new(fallback: Box<dyn Fn() -> T>) -> Self {
        Self { fallback }
    }
}

impl<'de, T> de::Visitor<'de> for DeserializeEmptyVecOrT<T>
where
    T: Deserialize<'de>,
{
    type Value = T;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("empty Vec or T")
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: de::SeqAccess<'de>,
    {
        if seq.next_element::<Self::Value>()?.is_none() {
            // It's an empty vec
            Ok((self.fallback)())
        } else {
            // not an empty vec
            Err(serde::de::Error::invalid_type(Unexpected::Seq, &self))
        }
    }

    fn visit_map<A>(self, map: A) -> Result<Self::Value, A::Error>
    where
        A: de::MapAccess<'de>,
    {
        Deserialize::deserialize(de::value::MapAccessDeserializer::new(map))
    }
}

struct DeserializeEmptyVecAsNone<T>(PhantomData<T>);

impl<'de, T> de::Visitor<'de> for DeserializeEmptyVecAsNone<T>
where
    T: Deserialize<'de>,
{
    type Value = Option<T>;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("empty Vec or T")
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: de::SeqAccess<'de>,
    {
        if seq.next_element::<T>()?.is_none() {
            // It's an empty vec
            Ok(None)
        } else {
            // not an empty vec
            Err(serde::de::Error::invalid_type(Unexpected::Seq, &self))
        }
    }

    fn visit_map<A>(self, map: A) -> Result<Self::Value, A::Error>
    where
        A: de::MapAccess<'de>,
    {
        T::deserialize(de::value::MapAccessDeserializer::new(map)).map(Some)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::*;
    use serde::Deserialize;

    #[derive(Debug, Deserialize, PartialEq, Eq)]
    pub struct OptionStructOrEmptyArray {
        #[serde(deserialize_with = "deserialize_empty_vec_as_none")]
        pub value: Option<OptionStructOrEmptyArrayInner>,
    }

    #[derive(Debug, Deserialize, PartialEq, Eq)]
    pub struct OptionStructOrEmptyArrayInner {
        foo: String,
    }

    #[rstest]
    #[case(r#"{"value": {"foo": "bar"}}"#, Some(OptionStructOrEmptyArrayInner { foo: "bar".to_string() }))]
    #[case(r#"{"value": []}"#, None)]
    fn test_deserialize_empty_vec_as_none(
        #[case] test_case: &str,
        #[case] expected_result: Option<OptionStructOrEmptyArrayInner>,
    ) {
        let option_struct: OptionStructOrEmptyArray =
            serde_json::from_str(test_case).expect("Test case should be a valid JSON");
        assert_eq!(expected_result, option_struct.value);
    }

    #[rstest]
    #[case(
        r#"{"value": ["not", "empty"]}"#,
        r#"invalid type: string "not", expected struct OptionStructOrEmptyArrayInner at line 1 column 16"#
    )]
    fn test_deserialize_empty_vec_as_none_errors(
        #[case] test_case: &str,
        #[case] expected_error_message: &str,
    ) {
        let result: Result<OptionStructOrEmptyArray, serde_json::Error> =
            serde_json::from_str(test_case);
        assert!(result.is_err(), "The deserializer should emit an error");
        assert_eq!(result.err().unwrap().to_string(), expected_error_message);
    }

    #[derive(Debug, Deserialize)]
    pub struct NullAsEmptyVec {
        #[serde(deserialize_with = "deserialize_null_as_empty_vec")]
        pub value: Vec<String>,
    }

    #[rstest]
    #[case(r#"{"value": ["string", "string2"]}"#, vec!["string".to_string(), "string2".to_string()])]
    #[case(r#"{"value": []}"#, vec![])]
    #[case(r#"{"value": null}"#, vec![])]
    fn test_deserialize_null_as_empty_vec(
        #[case] test_case: &str,
        #[case] expected_result: Vec<String>,
    ) {
        let null_as_empty_vec: NullAsEmptyVec =
            serde_json::from_str(test_case).expect("Test case should be a valid JSON");
        assert_eq!(expected_result, null_as_empty_vec.value);
    }

    #[rstest]
    #[case(
        r#"{"value": false}"#,
        r#"invalid type: boolean `false`, expected null or a sequence at line 1 column 15"#
    )]
    fn test_deserialize_null_as_empty_vec_errors(
        #[case] test_case: &str,
        #[case] expected_error_message: &str,
    ) {
        let null_as_empty_vec: Result<NullAsEmptyVec, serde_json::Error> =
            serde_json::from_str(test_case);
        assert!(
            null_as_empty_vec.is_err(),
            "The serializer should emit an error"
        );
        assert_eq!(
            null_as_empty_vec.err().unwrap().to_string(),
            expected_error_message
        );
    }

    #[derive(Debug, Deserialize, PartialEq, Eq)]
    struct Numbered {
        number: u32,
    }

    #[derive(Debug, Deserialize)]
    struct LenientList {
        items: LenientVec<Numbered>,
    }

    fn numbers(list: &LenientVec<Numbered>) -> Vec<u32> {
        list.valid.iter().map(|item| item.number).collect()
    }

    #[test]
    fn test_lenient_vec_reads_every_valid_element() {
        let list: LenientList =
            serde_json::from_str(r#"{"items": [{"number": 1}, {"number": 2}]}"#).unwrap();
        assert_eq!(numbers(&list.items), [1, 2]);
        assert!(list.items.invalid.is_empty());
    }

    #[test]
    fn test_lenient_vec_keeps_the_elements_around_an_invalid_one() {
        let json =
            r#"{"items": [{"number": 1}, {"number": -2, "name": "two"}, "three", {"number": 4}]}"#;
        let list: LenientList = serde_json::from_str(json).unwrap();

        assert_eq!(numbers(&list.items), [1, 4]);
        let invalid: Vec<_> = list
            .items
            .invalid
            .iter()
            .map(|element| (element.json.get(), element.error.to_string()))
            .collect();
        assert_eq!(
            invalid,
            [
                (
                    r#"{"number": -2, "name": "two"}"#,
                    "invalid value: integer `-2`, expected u32 at line 1 column 13".to_string()
                ),
                (
                    r#""three""#,
                    r#"invalid type: string "three", expected struct Numbered at line 1 column 7"#
                        .to_string()
                ),
            ]
        );
    }

    #[test]
    fn test_lenient_vec_of_an_empty_list() {
        let list: LenientList = serde_json::from_str(r#"{"items": []}"#).unwrap();
        assert!(list.items.valid.is_empty());
        assert!(list.items.invalid.is_empty());
    }

    #[rstest]
    #[case(
        r#"{"items": {"number": 1}}"#,
        "invalid type: map, expected a sequence at line 1 column 10"
    )]
    #[case(
        r#"{"items": null}"#,
        "invalid type: null, expected a sequence at line 1 column 14"
    )]
    #[case(
        r#"{"items": [{"number": 1}, {"number": ]}"#,
        "expected value at line 1 column 38"
    )]
    fn test_lenient_vec_errors(#[case] json: &str, #[case] expected_error_message: &str) {
        let result: Result<LenientList, serde_json::Error> = serde_json::from_str(json);
        assert_eq!(result.unwrap_err().to_string(), expected_error_message);
    }

    #[derive(Debug, Deserialize)]
    struct OptionalLenientList {
        #[serde(default)]
        items: Option<LenientVec<Numbered>>,
    }

    #[rstest]
    #[case(r#"{"items": null}"#, None)]
    #[case(r#"{}"#, None)]
    #[case(r#"{"items": [{"number": 1}, 2]}"#, Some(vec![1]))]
    fn test_optional_lenient_vec(#[case] json: &str, #[case] expected: Option<Vec<u32>>) {
        let list: OptionalLenientList = serde_json::from_str(json).unwrap();
        assert_eq!(list.items.as_ref().map(numbers), expected);
    }

    /// The JSON doesn't have to be text in memory: a reader and a `serde_json::Value` do too.
    #[test]
    fn test_lenient_vec_from_a_reader_and_from_a_value() {
        let json = r#"{"items": [{"number": 1}, "two", {"number": 3}]}"#;

        let list: LenientList = serde_json::from_reader(json.as_bytes()).unwrap();
        assert_eq!(numbers(&list.items), [1, 3]);
        assert_eq!(list.items.invalid.len(), 1);

        let value: serde_json::Value = serde_json::from_str(json).unwrap();
        let list: LenientList = serde_json::from_value(value).unwrap();
        assert_eq!(numbers(&list.items), [1, 3]);
        assert_eq!(list.items.invalid.len(), 1);
    }

    #[derive(Debug, Deserialize)]
    struct HoldsRawJson {
        payload: Box<RawValue>,
    }

    /// An element can hold raw JSON of its own.
    #[test]
    fn test_lenient_vec_of_elements_holding_raw_json() {
        let list: LenientVec<HoldsRawJson> =
            serde_json::from_str(r#"[{"payload": {"a": [1, 2]}}, {"payload": 3}, {}]"#).unwrap();
        let payloads: Vec<_> = list.valid.iter().map(|item| item.payload.get()).collect();
        assert_eq!(payloads, [r#"{"a": [1, 2]}"#, "3"]);
        assert_eq!(list.invalid.len(), 1);
    }

    #[derive(Debug, Deserialize)]
    struct BesideFlattened {
        items: LenientVec<Numbered>,
        #[serde(flatten)]
        rest: Numbered,
    }

    #[derive(Debug, Deserialize)]
    struct InsideFlattened {
        #[serde(flatten)]
        list: LenientList,
    }

    #[derive(Debug, Deserialize)]
    #[serde(untagged)]
    enum InsideUntagged {
        List(LenientList),
    }

    /// Next to a flattened field the list is read from the JSON itself. Inside one, or inside an
    /// untagged enum, it's read from serde's buffered copy, which is an error rather than a list
    /// that's quietly empty.
    #[test]
    fn test_lenient_vec_where_serde_buffers() {
        let json = r#"{"number": 7, "items": [{"number": 1}, "two"]}"#;

        let beside: BesideFlattened = serde_json::from_str(json).unwrap();
        assert_eq!(numbers(&beside.items), [1]);
        assert_eq!(beside.items.invalid.len(), 1);
        assert_eq!(beside.rest, Numbered { number: 7 });

        let inside = serde_json::from_str::<InsideFlattened>(json)
            .map(|inside| numbers(&inside.list.items))
            .unwrap_err();
        assert_eq!(
            inside.to_string(),
            "invalid type: newtype struct, expected any valid JSON value at line 1 column 46"
        );

        let untagged = serde_json::from_str::<InsideUntagged>(json)
            .map(|InsideUntagged::List(list)| numbers(&list.items))
            .unwrap_err();
        assert_eq!(
            untagged.to_string(),
            "data did not match any variant of untagged enum InsideUntagged"
        );
    }
}
