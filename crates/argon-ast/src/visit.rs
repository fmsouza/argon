//! Argon - AST visitor

use crate::*;

pub trait Visitor {
    fn visit_source_file(&mut self, source: &SourceFile) {
        for stmt in &source.statements {
            self.visit_stmt(stmt);
        }
    }

    fn visit_stmt(&mut self, stmt: &Stmt) {
        match stmt {
            Stmt::Block(b) => {
                for s in &b.statements {
                    self.visit_stmt(s);
                }
            }
            Stmt::If(i) => {
                self.visit_expr(&i.condition);
                self.visit_stmt(&i.consequent);
                if let Some(alt) = &i.alternate {
                    self.visit_stmt(alt);
                }
            }
            Stmt::Return(r) => {
                if let Some(arg) = &r.argument {
                    self.visit_expr(arg);
                }
            }
            _ => {}
        }
    }

    fn visit_expr(&mut self, expr: &Expr) {
        match expr {
            Expr::Binary(b) => {
                self.visit_expr(&b.left);
                self.visit_expr(&b.right);
            }
            Expr::Call(c) => {
                self.visit_expr(&c.callee);
                for arg in &c.arguments {
                    if let ExprOrSpread::Expr(e) = arg {
                        self.visit_expr(e);
                    }
                }
            }
            _ => {}
        }
    }
}
