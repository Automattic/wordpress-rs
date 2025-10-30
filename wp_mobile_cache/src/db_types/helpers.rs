use crate::{
    SqliteDbError,
    db_types::row_ext::{ColumnIndex, RowExt},
};
use rusqlite::Row;
use serde::{Deserialize, Serialize};
use std::str::FromStr;

/// Helper to get a required ID wrapper type (e.g., PostId, UserId) from a row.
pub fn get_id<T, C>(row: &Row, column: C) -> Result<T, SqliteDbError>
where
    T: From<i64>,
    C: ColumnIndex,
{
    let id: i64 = row.get_column(column)?;
    Ok(id.into())
}

/// Helper to get an optional ID wrapper type from a row.
pub fn get_optional_id<T, C>(row: &Row, column: C) -> Result<Option<T>, SqliteDbError>
where
    T: From<i64>,
    C: ColumnIndex,
{
    let id: Option<i64> = row.get_column(column)?;
    Ok(id.map(Into::into))
}

/// Helper to parse a required enum from a string column.
pub fn parse_enum<T, C>(row: &Row, column: C) -> Result<T, SqliteDbError>
where
    T: FromStr,
    T::Err: std::fmt::Display,
    C: ColumnIndex,
{
    let value_str: String = row.get_column(column)?;
    value_str
        .parse()
        .map_err(|e| SqliteDbError::SqliteError(format!("Failed to parse enum: {}", e)))
}

/// Helper to parse an optional enum from a string column.
pub fn parse_optional_enum<T, C>(row: &Row, column: C) -> Result<Option<T>, SqliteDbError>
where
    T: FromStr,
    T::Err: std::fmt::Display,
    C: ColumnIndex,
{
    let value_str: Option<String> = row.get_column(column)?;
    value_str
        .map(|s| s.parse())
        .transpose()
        .map_err(|e| SqliteDbError::SqliteError(format!("Failed to parse enum: {}", e)))
}

/// Helper to deserialize a JSON array from a TEXT column.
pub fn deserialize_json_array<T>(json_str: Option<String>) -> Result<Option<Vec<T>>, SqliteDbError>
where
    T: for<'de> Deserialize<'de>,
{
    match json_str {
        Some(s) => {
            let items: Vec<T> = serde_json::from_str(&s)
                .map_err(|e| SqliteDbError::SqliteError(format!("Failed to parse JSON: {}", e)))?;
            Ok(Some(items))
        }
        None => Ok(None),
    }
}

/// Helper to serialize a vector to a JSON string.
pub fn serialize_to_json<T>(items: &Option<Vec<T>>) -> Result<Option<String>, SqliteDbError>
where
    T: Serialize,
{
    items
        .as_ref()
        .map(serde_json::to_string)
        .transpose()
        .map_err(|e| SqliteDbError::SqliteError(format!("Failed to serialize to JSON: {}", e)))
}

/// Helper to serialize a value to a JSON string.
pub fn serialize_value_to_json<T>(value: &Option<T>) -> Result<Option<String>, SqliteDbError>
where
    T: Serialize,
{
    value
        .as_ref()
        .map(serde_json::to_string)
        .transpose()
        .map_err(|e| SqliteDbError::SqliteError(format!("Failed to serialize to JSON: {}", e)))
}

/// Helper to deserialize a JSON object from a TEXT column.
pub fn deserialize_json_value<T>(json_str: Option<String>) -> Result<Option<T>, SqliteDbError>
where
    T: for<'de> Deserialize<'de>,
{
    json_str
        .map(|s| serde_json::from_str(&s))
        .transpose()
        .map_err(|e| SqliteDbError::SqliteError(format!("Failed to parse JSON: {}", e)))
}

/// Helper to convert an INTEGER to a boolean (0 = false, non-zero = true).
pub fn integer_to_bool(value: Option<i64>) -> Option<bool> {
    value.map(|v| v != 0)
}

/// Helper to convert a boolean to an INTEGER (false = 0, true = 1).
pub fn bool_to_integer(value: Option<bool>) -> Option<i64> {
    value.map(|b| if b { 1 } else { 0 })
}

/// Helper to parse a required DateTime-like type from a string column.
pub fn parse_datetime<T, C>(row: &Row, column: C) -> Result<T, SqliteDbError>
where
    T: FromStr,
    T::Err: std::fmt::Display,
    C: ColumnIndex,
{
    let datetime_str: String = row.get_column(column)?;
    datetime_str
        .parse()
        .map_err(|e| SqliteDbError::SqliteError(format!("Failed to parse datetime: {}", e)))
}

/// Helper to deserialize a JSON array of ID wrapper types.
/// This handles the case where we store `Vec<TermId>` as `Vec<i64>` in JSON.
pub fn deserialize_json_id_array<T>(
    json_str: Option<String>,
) -> Result<Option<Vec<T>>, SqliteDbError>
where
    T: From<i64>,
{
    match json_str {
        Some(s) => {
            let ids: Vec<i64> = serde_json::from_str(&s)
                .map_err(|e| SqliteDbError::SqliteError(format!("Failed to parse JSON: {}", e)))?;
            Ok(Some(ids.into_iter().map(Into::into).collect()))
        }
        None => Ok(None),
    }
}

/// Helper to serialize a vector of ID wrapper types to JSON.
/// This handles the case where we store `Vec<TermId>` as `Vec<i64>` in JSON.
pub fn serialize_json_id_array<T, F>(
    items: &Option<Vec<T>>,
    to_i64: F,
) -> Result<Option<String>, SqliteDbError>
where
    F: Fn(&T) -> i64,
{
    items
        .as_ref()
        .map(|items| {
            let ids: Vec<i64> = items.iter().map(to_i64).collect();
            serde_json::to_string(&ids)
        })
        .transpose()
        .map_err(|e| SqliteDbError::SqliteError(format!("Failed to serialize to JSON: {}", e)))
}
