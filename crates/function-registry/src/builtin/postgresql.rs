// Copyright (c) 2025 woxQAQ
//
// Licensed under the MIT License or Apache License 2.0
// See LICENSE files for details

//! PostgreSQL builtin function definitions

use crate::{DataType, FunctionMetadata, FunctionType};

/// Get all builtin PostgreSQL functions
pub fn all_functions() -> Vec<FunctionMetadata> {
    vec![
        // Aggregate functions
        FunctionMetadata::new("COUNT", DataType::BigInt)
            .with_type(FunctionType::Aggregate)
            .with_description("Count the number of rows"),
        FunctionMetadata::new("SUM", DataType::Decimal)
            .with_type(FunctionType::Aggregate)
            .with_description("Sum of values"),
        FunctionMetadata::new("AVG", DataType::Decimal)
            .with_type(FunctionType::Aggregate)
            .with_description("Average of values"),
        FunctionMetadata::new("MIN", DataType::Text)
            .with_type(FunctionType::Aggregate)
            .with_description("Minimum value"),
        FunctionMetadata::new("MAX", DataType::Text)
            .with_type(FunctionType::Aggregate)
            .with_description("Maximum value"),
        FunctionMetadata::new("STRING_AGG", DataType::Text)
            .with_type(FunctionType::Aggregate)
            .with_description("Concatenate values with delimiter"),
        FunctionMetadata::new("ARRAY_AGG", DataType::Other("array".to_string()))
            .with_type(FunctionType::Aggregate)
            .with_description("Collect values into an array"),
        FunctionMetadata::new("JSON_AGG", DataType::Json)
            .with_type(FunctionType::Aggregate)
            .with_description("Aggregate values as JSON"),
        FunctionMetadata::new("JSONB_AGG", DataType::Json)
            .with_type(FunctionType::Aggregate)
            .with_description("Aggregate values as JSONB"),
        // Scalar functions
        FunctionMetadata::new("ABS", DataType::Decimal)
            .with_type(FunctionType::Scalar)
            .with_description("Absolute value"),
        FunctionMetadata::new("CEIL", DataType::Integer)
            .with_type(FunctionType::Scalar)
            .with_description("Round up to nearest integer"),
        FunctionMetadata::new("FLOOR", DataType::Integer)
            .with_type(FunctionType::Scalar)
            .with_description("Round down to nearest integer"),
        FunctionMetadata::new("ROUND", DataType::Decimal)
            .with_type(FunctionType::Scalar)
            .with_description("Round to nearest decimal"),
        FunctionMetadata::new("TRUNC", DataType::Decimal)
            .with_type(FunctionType::Scalar)
            .with_description("Truncate decimal"),
        FunctionMetadata::new("CONCAT", DataType::Text)
            .with_type(FunctionType::Scalar)
            .with_description("Concatenate strings"),
        FunctionMetadata::new("SUBSTRING", DataType::Text)
            .with_type(FunctionType::Scalar)
            .with_description("Extract substring"),
        FunctionMetadata::new("LENGTH", DataType::Integer)
            .with_type(FunctionType::Scalar)
            .with_description("String length"),
        FunctionMetadata::new("UPPER", DataType::Text)
            .with_type(FunctionType::Scalar)
            .with_description("Convert to uppercase"),
        FunctionMetadata::new("LOWER", DataType::Text)
            .with_type(FunctionType::Scalar)
            .with_description("Convert to lowercase"),
        FunctionMetadata::new("TRIM", DataType::Text)
            .with_type(FunctionType::Scalar)
            .with_description("Remove leading/trailing whitespace"),
        FunctionMetadata::new("LTRIM", DataType::Text)
            .with_type(FunctionType::Scalar)
            .with_description("Remove leading whitespace"),
        FunctionMetadata::new("RTRIM", DataType::Text)
            .with_type(FunctionType::Scalar)
            .with_description("Remove trailing whitespace"),
        FunctionMetadata::new("COALESCE", DataType::Text)
            .with_type(FunctionType::Scalar)
            .with_description("Return first non-null value"),
        FunctionMetadata::new("NULLIF", DataType::Text)
            .with_type(FunctionType::Scalar)
            .with_description("Return NULL if arguments are equal"),
        FunctionMetadata::new("GREATEST", DataType::Text)
            .with_type(FunctionType::Scalar)
            .with_description("Return largest value"),
        FunctionMetadata::new("LEAST", DataType::Text)
            .with_type(FunctionType::Scalar)
            .with_description("Return smallest value"),
        FunctionMetadata::new("POSITION", DataType::Integer)
            .with_type(FunctionType::Scalar)
            .with_description("Position of substring"),
        FunctionMetadata::new("STRPOS", DataType::Integer)
            .with_type(FunctionType::Scalar)
            .with_description("Position of substring"),
        FunctionMetadata::new("REPLACE", DataType::Text)
            .with_type(FunctionType::Scalar)
            .with_description("Replace occurrences"),
        FunctionMetadata::new("SPLIT_PART", DataType::Text)
            .with_type(FunctionType::Scalar)
            .with_description("Split string and return field"),
        FunctionMetadata::new("REGEXP_REPLACE", DataType::Text)
            .with_type(FunctionType::Scalar)
            .with_description("Replace using regex"),
        FunctionMetadata::new("REGEXP_MATCHES", DataType::Other("array".to_string()))
            .with_type(FunctionType::Scalar)
            .with_description("Match regex and return array"),
        // Date/Time functions
        FunctionMetadata::new("NOW", DataType::Timestamp)
            .with_type(FunctionType::Scalar)
            .with_description("Current date and time"),
        FunctionMetadata::new("CURRENT_DATE", DataType::Date)
            .with_type(FunctionType::Scalar)
            .with_description("Current date"),
        FunctionMetadata::new("CURRENT_TIME", DataType::Time)
            .with_type(FunctionType::Scalar)
            .with_description("Current time"),
        FunctionMetadata::new("CURRENT_TIMESTAMP", DataType::Timestamp)
            .with_type(FunctionType::Scalar)
            .with_description("Current date and time"),
        FunctionMetadata::new("AGE", DataType::Other("interval".to_string()))
            .with_type(FunctionType::Scalar)
            .with_description("Calculate interval"),
        FunctionMetadata::new("DATE_TRUNC", DataType::Timestamp)
            .with_type(FunctionType::Scalar)
            .with_description("Truncate to precision"),
        FunctionMetadata::new("DATE_PART", DataType::Float)
            .with_type(FunctionType::Scalar)
            .with_description("Extract date part"),
        FunctionMetadata::new("EXTRACT", DataType::Float)
            .with_type(FunctionType::Scalar)
            .with_description("Extract date/time field"),
        FunctionMetadata::new("TO_DATE", DataType::Date)
            .with_type(FunctionType::Scalar)
            .with_description("Convert string to date"),
        FunctionMetadata::new("TO_TIMESTAMP", DataType::Timestamp)
            .with_type(FunctionType::Scalar)
            .with_description("Convert string to timestamp"),
        // Window functions (PostgreSQL 8.4+)
        FunctionMetadata::new("ROW_NUMBER", DataType::BigInt)
            .with_type(FunctionType::Window)
            .with_description("Row number within partition"),
        FunctionMetadata::new("RANK", DataType::BigInt)
            .with_type(FunctionType::Window)
            .with_description("Rank within partition"),
        FunctionMetadata::new("DENSE_RANK", DataType::BigInt)
            .with_type(FunctionType::Window)
            .with_description("Dense rank within partition"),
        FunctionMetadata::new("NTILE", DataType::Integer)
            .with_type(FunctionType::Window)
            .with_description("Divide rows into buckets"),
        FunctionMetadata::new("LAG", DataType::Text)
            .with_type(FunctionType::Window)
            .with_description("Value from previous row"),
        FunctionMetadata::new("LEAD", DataType::Text)
            .with_type(FunctionType::Window)
            .with_description("Value from next row"),
        FunctionMetadata::new("FIRST_VALUE", DataType::Text)
            .with_type(FunctionType::Window)
            .with_description("First value in window"),
        FunctionMetadata::new("LAST_VALUE", DataType::Text)
            .with_type(FunctionType::Window)
            .with_description("Last value in window"),
        FunctionMetadata::new("NTH_VALUE", DataType::Text)
            .with_type(FunctionType::Window)
            .with_description("Nth value in window"),
        // JSON functions (PostgreSQL 9.2+)
        FunctionMetadata::new("TO_JSON", DataType::Json)
            .with_type(FunctionType::Scalar)
            .with_description("Convert to JSON"),
        FunctionMetadata::new("TO_JSONB", DataType::Json)
            .with_type(FunctionType::Scalar)
            .with_description("Convert to JSONB"),
        FunctionMetadata::new("JSON_BUILD_OBJECT", DataType::Json)
            .with_type(FunctionType::Scalar)
            .with_description("Build JSON object"),
        FunctionMetadata::new("JSONB_BUILD_OBJECT", DataType::Json)
            .with_type(FunctionType::Scalar)
            .with_description("Build JSONB object"),
        FunctionMetadata::new("JSON_ARRAY_ELEMENTS", DataType::Json)
            .with_type(FunctionType::Scalar)
            .with_description("Expand JSON array"),
        FunctionMetadata::new("JSONB_ARRAY_ELEMENTS", DataType::Json)
            .with_type(FunctionType::Scalar)
            .with_description("Expand JSONB array"),
        // Array functions
        FunctionMetadata::new("ARRAY_LENGTH", DataType::Integer)
            .with_type(FunctionType::Scalar)
            .with_description("Get array length"),
        FunctionMetadata::new("UNNEST", DataType::Text)
            .with_type(FunctionType::Scalar)
            .with_description("Expand array to rows"),
        FunctionMetadata::new("ARRAY_APPEND", DataType::Other("array".to_string()))
            .with_type(FunctionType::Scalar)
            .with_description("Append element to array"),
        FunctionMetadata::new("ARRAY_PREPEND", DataType::Other("array".to_string()))
            .with_type(FunctionType::Scalar)
            .with_description("Prepend element to array"),
        FunctionMetadata::new("ARRAY_CAT", DataType::Other("array".to_string()))
            .with_type(FunctionType::Scalar)
            .with_description("Concatenate arrays"),
        // Mathematical functions
        FunctionMetadata::new("SQRT", DataType::Decimal)
            .with_type(FunctionType::Scalar)
            .with_description("Square root"),
        FunctionMetadata::new("POWER", DataType::Decimal)
            .with_type(FunctionType::Scalar)
            .with_description("Raise to power"),
        FunctionMetadata::new("EXP", DataType::Decimal)
            .with_type(FunctionType::Scalar)
            .with_description("Exponential"),
        FunctionMetadata::new("LN", DataType::Decimal)
            .with_type(FunctionType::Scalar)
            .with_description("Natural logarithm"),
        FunctionMetadata::new("LOG", DataType::Decimal)
            .with_type(FunctionType::Scalar)
            .with_description("Logarithm"),
        FunctionMetadata::new("MOD", DataType::Decimal)
            .with_type(FunctionType::Scalar)
            .with_description("Modulus"),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_has_count() {
        let funcs = all_functions();
        assert!(funcs.iter().any(|f| f.name == "COUNT"));
    }

    #[test]
    fn test_has_postgresql_specific_functions() {
        let funcs = all_functions();
        assert!(funcs.iter().any(|f| f.name == "STRING_AGG"));
        assert!(funcs.iter().any(|f| f.name == "JSON_AGG"));
    }

    #[test]
    fn test_has_array_functions() {
        let funcs = all_functions();
        assert!(funcs.iter().any(|f| f.name == "ARRAY_LENGTH"));
        assert!(funcs.iter().any(|f| f.name == "UNNEST"));
    }

    #[test]
    fn test_has_json_functions() {
        let funcs = all_functions();
        assert!(funcs.iter().any(|f| f.name == "TO_JSON"));
        assert!(funcs.iter().any(|f| f.name == "JSON_BUILD_OBJECT"));
    }

    #[test]
    fn test_string_agg_is_aggregate() {
        let funcs = all_functions();
        let string_agg = funcs.iter().find(|f| f.name == "STRING_AGG").unwrap();
        assert!(matches!(string_agg.function_type, FunctionType::Aggregate));
    }
}
