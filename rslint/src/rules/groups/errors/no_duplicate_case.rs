use crate::cst_rule;
use crate::diagnostic::DiagnosticBuilder;
use crate::visit::{Node, Visit};
use rslint_parse::parser::cst::stmt::*;
use rslint_parse::span::Span;
use std::collections::HashMap;

cst_rule! {
    "no-duplicate-case",
    NoDuplicateCase
}

impl Visit for NoDuplicateCaseVisitor<'_, '_> {
    fn visit_switch_stmt(&mut self, switch: &SwitchStmt, _parent: &dyn Node) {
        let mut map: HashMap<&str, Span> = HashMap::new();

        for case in switch.cases.iter() {
            if let Some(ref test) = case.test {
                let code_str = test.span().content(self.ctx.file_source);
                if map.contains_key(code_str) {
                    let builder = DiagnosticBuilder::error(
                        self.ctx.file_id,
                        "no-duplicate-case",
                        &format!(
                            "`{}` is redundantly tested for in a switch statement",
                            code_str
                        ),
                    )
                    .secondary(map.get(code_str).unwrap().to_owned(), &format!("`{}` is first tested for here", code_str))
                    .primary(test.span().to_owned(), &format!("`{}` is again checked for here", code_str));

                    self.ctx.diagnostics.push(builder.into());
                } else {
                    map.insert(test.span().content(self.ctx.file_source), test.span().to_owned());
                }
            }
            for stmt in case.cons.iter() {
                self.visit_stmt(stmt, _parent);
            }
        }
    }
}
