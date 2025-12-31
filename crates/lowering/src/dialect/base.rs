// Copyright (c) 2025 woxQAQ
//
// Licensed under the MIT License or Apache License 2.0
// See LICENSE files for details

//! Base trait providing shared lowering logic for all dialects

use crate::{CstNode, LoweringContext, LoweringError, LoweringResult};

/// Base trait providing shared lowering logic
///
/// This trait provides common helper methods that are useful across
/// all dialect implementations.
pub trait DialectLoweringBase<N>
where
    N: CstNode,
{
    /// Extract a required child node, returning an error if not found
    fn require_child<'a>(
        &self,
        _ctx: &mut LoweringContext,
        node: &'a N,
        field: &str,
    ) -> Result<&'a N, LoweringError> {
        let children = node.children(field);
        match children.first() {
            Some(&child) => Ok(child),
            None => Err(LoweringError::MissingChild {
                context: node.kind().to_string(),
                expected: field.to_string(),
            }),
        }
    }

    /// Extract an optional child node, returning None if not found
    fn optional_child<'a>(&self, node: &'a N, field: &str) -> Option<&'a N> {
        let children = node.children(field);
        children.first().copied()
    }

    /// Lower a list of child nodes
    fn lower_children<T, F>(
        &self,
        ctx: &mut LoweringContext,
        nodes: &[&N],
        mut lower_fn: F,
    ) -> LoweringResult<Vec<T>>
    where
        F: FnMut(&mut LoweringContext, &N) -> LoweringResult<T>,
    {
        let mut results = Vec::new();
        for &node in nodes {
            results.push(lower_fn(ctx, node)?);
        }
        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cst::MockCstNode;
    use unified_sql_lsp_ir::Dialect;

    struct TestLoweringImpl;

    impl DialectLoweringBase<MockCstNode> for TestLoweringImpl {}

    #[test]
    fn test_require_child_success() {
        let impl_obj = TestLoweringImpl;
        let mut ctx = LoweringContext::new(Dialect::MySQL);

        let child = MockCstNode::new("column_ref");
        let node = MockCstNode::new("select_list").with_child(Some("item"), child);

        let result = impl_obj.require_child(&mut ctx, &node, "item");
        assert!(result.is_ok());
        assert_eq!(result.unwrap().kind(), "column_ref");
    }

    #[test]
    fn test_require_child_missing() {
        let impl_obj = TestLoweringImpl;
        let mut ctx = LoweringContext::new(Dialect::MySQL);

        let node = MockCstNode::new("select_list");

        let result = impl_obj.require_child(&mut ctx, &node, "item");
        assert!(result.is_err());
        if let Err(LoweringError::MissingChild { expected, context }) = result {
            assert_eq!(expected, "item");
            assert_eq!(context, "select_list");
        } else {
            panic!("Expected MissingChild error");
        }
    }

    #[test]
    fn test_optional_child_present() {
        let impl_obj = TestLoweringImpl;

        let child = MockCstNode::new("column_ref");
        let node = MockCstNode::new("select_list").with_child(Some("item"), child);

        let result = impl_obj.optional_child(&node, "item");
        assert!(result.is_some());
        assert_eq!(result.unwrap().kind(), "column_ref");
    }

    #[test]
    fn test_optional_child_missing() {
        let impl_obj = TestLoweringImpl;

        let node = MockCstNode::new("select_list");

        let result = impl_obj.optional_child(&node, "item");
        assert!(result.is_none());
    }

    #[test]
    fn test_lower_children() {
        let impl_obj = TestLoweringImpl;
        let mut ctx = LoweringContext::new(Dialect::MySQL);

        let node1 = MockCstNode::new("column_ref");
        let node2 = MockCstNode::new("column_ref");
        let node3 = MockCstNode::new("literal");

        let nodes = vec![&node1, &node2, &node3];

        let result =
            impl_obj.lower_children(&mut ctx, &nodes, |_, node| Ok(node.kind().to_string()));

        assert!(result.is_ok());
        let kinds = result.unwrap();
        assert_eq!(kinds, vec!["column_ref", "column_ref", "literal"]);
    }
}
