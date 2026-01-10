// Copyright (c) 2025 woxQAQ
//
// Licensed under the MIT License or Apache License 2.0
// See LICENSE files for details

//! MySQL builtin function definitions

use crate::{DataType, FunctionMetadata, FunctionType};

/// Get all builtin MySQL functions
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
        FunctionMetadata::new("MIN", DataType::Decimal)
            .with_type(FunctionType::Aggregate)
            .with_description("Minimum value"),
        FunctionMetadata::new("MAX", DataType::Decimal)
            .with_type(FunctionType::Aggregate)
            .with_description("Maximum value"),
        FunctionMetadata::new("GROUP_CONCAT", DataType::Text)
            .with_type(FunctionType::Aggregate)
            .with_description("Concatenate values from multiple rows"),
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
        FunctionMetadata::new("COALESCE", DataType::Text)
            .with_type(FunctionType::Scalar)
            .with_description("Return first non-null value"),
        FunctionMetadata::new("IFNULL", DataType::Text)
            .with_type(FunctionType::Scalar)
            .with_description("Return alternative if null"),
        // Date/Time functions
        FunctionMetadata::new("NOW", DataType::DateTime)
            .with_type(FunctionType::Scalar)
            .with_description("Current date and time"),
        FunctionMetadata::new("CURDATE", DataType::Date)
            .with_type(FunctionType::Scalar)
            .with_description("Current date"),
        FunctionMetadata::new("CURTIME", DataType::Time)
            .with_type(FunctionType::Scalar)
            .with_description("Current time"),
        FunctionMetadata::new("DATE_FORMAT", DataType::Text)
            .with_type(FunctionType::Scalar)
            .with_description("Format date/time"),
        FunctionMetadata::new("DATE_ADD", DataType::DateTime)
            .with_type(FunctionType::Scalar)
            .with_description("Add time interval"),
        FunctionMetadata::new("DATE_SUB", DataType::DateTime)
            .with_type(FunctionType::Scalar)
            .with_description("Subtract time interval"),
        FunctionMetadata::new("DATEDIFF", DataType::Integer)
            .with_type(FunctionType::Scalar)
            .with_description("Difference between dates"),
        // Window functions (MySQL 8.0+)
        FunctionMetadata::new("ROW_NUMBER", DataType::BigInt)
            .with_type(FunctionType::Window)
            .with_description("Row number within partition"),
        FunctionMetadata::new("RANK", DataType::BigInt)
            .with_type(FunctionType::Window)
            .with_description("Rank within partition"),
        FunctionMetadata::new("DENSE_RANK", DataType::BigInt)
            .with_type(FunctionType::Window)
            .with_description("Dense rank within partition"),
        FunctionMetadata::new("LAG", DataType::Text)
            .with_type(FunctionType::Window)
            .with_description("Value from previous row"),
        FunctionMetadata::new("LEAD", DataType::Text)
            .with_type(FunctionType::Window)
            .with_description("Value from next row"),
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
    fn test_has_aggregate_functions() {
        let funcs = all_functions();
        assert!(funcs.iter().any(|f| f.name == "SUM"));
        assert!(funcs.iter().any(|f| f.name == "AVG"));
        assert!(funcs.iter().any(|f| f.name == "MIN"));
        assert!(funcs.iter().any(|f| f.name == "MAX"));
    }

    #[test]
    fn test_has_scalar_functions() {
        let funcs = all_functions();
        assert!(funcs.iter().any(|f| f.name == "ABS"));
        assert!(funcs.iter().any(|f| f.name == "CONCAT"));
        assert!(funcs.iter().any(|f| f.name == "UPPER"));
    }

    #[test]
    fn test_has_window_functions() {
        let funcs = all_functions();
        assert!(funcs.iter().any(|f| f.name == "ROW_NUMBER"));
        assert!(funcs.iter().any(|f| f.name == "RANK"));
        assert!(funcs.iter().any(|f| f.name == "LAG"));
    }

    #[test]
    fn test_count_is_aggregate() {
        let funcs = all_functions();
        let count_func = funcs.iter().find(|f| f.name == "COUNT").unwrap();
        assert!(matches!(count_func.function_type, FunctionType::Aggregate));
    }

    #[test]
    fn test_row_number_is_window() {
        let funcs = all_functions();
        let row_num = funcs.iter().find(|f| f.name == "ROW_NUMBER").unwrap();
        assert!(matches!(row_num.function_type, FunctionType::Window));
    }
}
