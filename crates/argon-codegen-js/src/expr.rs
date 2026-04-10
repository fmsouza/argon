use argon_ast::*;

use super::*;

impl JsCodegen {
    pub(crate) fn generate_expression(&mut self, expr: &Expr) -> Result<(), CodegenError> {
        match expr {
            Expr::Literal(lit) => self.generate_literal(lit),
            Expr::Identifier(id) => {
                self.output.push_str(&id.sym);
                Ok(())
            }
            Expr::Binary(b) => self.generate_binary(b),
            Expr::Unary(u) => self.generate_unary(u),
            Expr::Call(c) => self.generate_call(c),
            Expr::Member(m) => self.generate_member(m),
            Expr::Assignment(a) => self.generate_assignment(a),
            Expr::This(_) => {
                self.output.push_str("this");
                Ok(())
            }
            Expr::Await(a) => {
                self.output.push_str("await ");
                self.generate_expression(&a.argument)?;
                Ok(())
            }
            Expr::New(n) => {
                self.output.push_str("new ");
                self.generate_expression(&n.callee)?;
                self.output.push('(');
                self.generate_args(&n.arguments)?;
                self.output.push(')');
                Ok(())
            }
            Expr::Conditional(c) => {
                self.output.push('(');
                self.generate_expression(&c.test)?;
                self.output.push_str(" ? ");
                self.generate_expression(&c.consequent)?;
                self.output.push_str(" : ");
                self.generate_expression(&c.alternate)?;
                self.output.push(')');
                Ok(())
            }
            Expr::Object(o) => self.generate_object(o),
            Expr::Ref(r) => {
                self.generate_expression(&r.expr)?;
                Ok(())
            }
            Expr::MutRef(r) => {
                self.generate_expression(&r.expr)?;
                Ok(())
            }
            Expr::JsxElement(e) => self.generate_jsx_element(e),
            Expr::JsxFragment(f) => self.generate_jsx_fragment(f),
            _ => {
                self.output.push_str("undefined");
                Ok(())
            }
        }
    }

    fn generate_literal(&mut self, lit: &Literal) -> Result<(), CodegenError> {
        match lit {
            Literal::Number(n) => {
                self.output.push_str(&n.raw);
                Ok(())
            }
            Literal::String(s) => {
                self.output.push_str(&s.value);
                Ok(())
            }
            Literal::Boolean(b) => {
                self.output.push_str(if b.value { "true" } else { "false" });
                Ok(())
            }
            Literal::Null(_) => {
                self.output.push_str("null");
                Ok(())
            }
            _ => Ok(()),
        }
    }

    fn generate_binary(&mut self, b: &BinaryExpr) -> Result<(), CodegenError> {
        self.output.push('(');
        self.generate_expression(&b.left)?;

        let op = match b.operator {
            BinaryOperator::Plus => "+",
            BinaryOperator::Minus => "-",
            BinaryOperator::Multiply => "*",
            BinaryOperator::Divide => "/",
            BinaryOperator::Modulo => "%",
            BinaryOperator::Equal => "===",
            BinaryOperator::NotEqual => "!==",
            BinaryOperator::StrictEqual => "===",
            BinaryOperator::StrictNotEqual => "!==",
            BinaryOperator::LessThan => "<",
            BinaryOperator::LessThanOrEqual => "<=",
            BinaryOperator::GreaterThan => ">",
            BinaryOperator::GreaterThanOrEqual => ">=",
            BinaryOperator::BitwiseAnd => "&",
            BinaryOperator::BitwiseOr => "|",
            BinaryOperator::BitwiseXor => "^",
            BinaryOperator::LeftShift => "<<",
            BinaryOperator::RightShift => ">>",
            _ => " + ",
        };

        self.output.push_str(&format!(" {} ", op));
        self.generate_expression(&b.right)?;
        self.output.push(')');
        Ok(())
    }

    fn generate_unary(&mut self, u: &UnaryExpr) -> Result<(), CodegenError> {
        let op = match u.operator {
            UnaryOperator::Minus => "-",
            UnaryOperator::Plus => "+",
            UnaryOperator::LogicalNot => "!",
            UnaryOperator::BitwiseNot => "~",
            UnaryOperator::Typeof => "typeof ",
            _ => "",
        };
        self.output.push_str(op);
        self.generate_expression(&u.argument)?;
        Ok(())
    }

    pub(crate) fn generate_args(&mut self, args: &[ExprOrSpread]) -> Result<(), CodegenError> {
        for (i, arg) in args.iter().enumerate() {
            if i > 0 {
                self.output.push_str(", ");
            }
            match arg {
                ExprOrSpread::Expr(e) => self.generate_expression(e)?,
                ExprOrSpread::Named { value, .. } => self.generate_expression(value)?,
                ExprOrSpread::Spread(s) => {
                    self.output.push_str("...");
                    self.generate_expression(&s.argument)?;
                }
            }
        }
        Ok(())
    }

    fn generate_call(&mut self, c: &CallExpr) -> Result<(), CodegenError> {
        // Check if this is a Some(), None, Ok(), or Err() call
        if let Expr::Identifier(id) = c.callee.as_ref() {
            match id.sym.as_str() {
                "Some" => {
                    self.output.push_str("new Argon.Option.Some(");
                    self.generate_args(&c.arguments)?;
                    self.output.push(')');
                    return Ok(());
                }
                "None" => {
                    self.output.push_str("new Argon.Option.None()");
                    return Ok(());
                }
                "Ok" => {
                    self.output.push_str("new Argon.Result.Ok(");
                    self.generate_args(&c.arguments)?;
                    self.output.push(')');
                    return Ok(());
                }
                "Err" => {
                    self.output.push_str("new Argon.Result.Err(");
                    self.generate_args(&c.arguments)?;
                    self.output.push(')');
                    return Ok(());
                }
                "Vec" => {
                    self.output.push_str("new Argon.Vec(");
                    self.generate_args(&c.arguments)?;
                    self.output.push(')');
                    return Ok(());
                }
                "Shared" => {
                    self.output.push_str("Argon.Shared.wrap(");
                    self.generate_args(&c.arguments)?;
                    self.output.push(')');
                    return Ok(());
                }
                _ => {}
            }
        }

        // Check if this is a Argon.Option.Some, Argon.Result.Ok, etc. call
        if let Expr::Member(m) = c.callee.as_ref() {
            if let Expr::Member(outer) = m.object.as_ref() {
                if let Expr::Identifier(obj_id) = outer.object.as_ref() {
                    if obj_id.sym == "Argon" {
                        if let Expr::Identifier(inner_id) = outer.property.as_ref() {
                            if let Expr::Identifier(method_id) = m.property.as_ref() {
                                if inner_id.sym == "Option" {
                                    match method_id.sym.as_str() {
                                        "Some" => {
                                            self.output.push_str("new Argon.Option.Some(");
                                            self.generate_args(&c.arguments)?;
                                            self.output.push(')');
                                            return Ok(());
                                        }
                                        "None" => {
                                            self.output.push_str("new Argon.Option.None()");
                                            return Ok(());
                                        }
                                        _ => {}
                                    }
                                }
                                if inner_id.sym == "Result" {
                                    match method_id.sym.as_str() {
                                        "Ok" => {
                                            self.output.push_str("new Argon.Result.Ok(");
                                            self.generate_args(&c.arguments)?;
                                            self.output.push(')');
                                            return Ok(());
                                        }
                                        "Err" => {
                                            self.output.push_str("new Argon.Result.Err(");
                                            self.generate_args(&c.arguments)?;
                                            self.output.push(')');
                                            return Ok(());
                                        }
                                        _ => {}
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        self.generate_expression(&c.callee)?;
        self.output.push('(');
        self.generate_args(&c.arguments)?;
        self.output.push(')');
        Ok(())
    }

    pub(crate) fn generate_object(&mut self, o: &ObjectExpression) -> Result<(), CodegenError> {
        self.output.push_str("{ ");
        for (i, prop) in o.properties.iter().enumerate() {
            if i > 0 {
                self.output.push_str(", ");
            }
            match prop {
                ObjectProperty::Property(p) => {
                    self.generate_expression(&p.key)?;
                    self.output.push_str(": ");
                    match &p.value {
                        ExprOrSpread::Expr(e) => {
                            self.generate_expression(e)?;
                        }
                        ExprOrSpread::Named { value, .. } => {
                            self.generate_expression(value.as_ref())?;
                        }
                        ExprOrSpread::Spread(_) => {}
                    }
                }
                ObjectProperty::Shorthand(id) => {
                    self.output.push_str(&id.sym);
                }
                _ => {}
            }
        }
        self.output.push_str(" }");
        Ok(())
    }

    fn generate_member(&mut self, m: &MemberExpr) -> Result<(), CodegenError> {
        self.generate_expression(&m.object)?;

        if m.computed {
            self.output.push('[');
            self.generate_expression(&m.property)?;
            self.output.push(']');
        } else {
            self.output.push('.');
            self.generate_expression(&m.property)?;
        }
        Ok(())
    }

    fn generate_assignment(&mut self, a: &AssignmentExpr) -> Result<(), CodegenError> {
        self.generate_assignment_target(&a.left)?;

        let op = match a.operator {
            AssignmentOperator::Assign => "=",
            AssignmentOperator::PlusAssign => "+=",
            AssignmentOperator::MinusAssign => "-=",
            AssignmentOperator::MultiplyAssign => "*=",
            AssignmentOperator::DivideAssign => "/=",
            _ => "=",
        };

        self.output.push_str(&format!(" {} ", op));
        self.generate_expression(&a.right)?;
        Ok(())
    }

    fn generate_assignment_target(
        &mut self,
        target: &AssignmentTarget,
    ) -> Result<(), CodegenError> {
        match target {
            AssignmentTarget::Simple(expr) => {
                self.generate_expression(expr)?;
            }
            AssignmentTarget::Member(member) => {
                self.generate_expression(&member.object)?;
                self.output.push('.');
                self.generate_expression(&member.property)?;
            }
            _ => {
                self.output.push_str("/* target */");
            }
        }
        Ok(())
    }
}
