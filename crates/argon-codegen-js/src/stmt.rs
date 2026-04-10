use argon_ast::*;

use super::*;

impl JsCodegen {
    pub(crate) fn generate_statement(&mut self, stmt: &Stmt) -> Result<(), CodegenError> {
        match stmt {
            Stmt::Variable(v) => self.generate_variable(v),
            Stmt::Function(f) if f.is_intrinsic => Ok(()),
            Stmt::Function(f) => self.generate_function(f),
            Stmt::AsyncFunction(f) if f.is_intrinsic => Ok(()),
            Stmt::AsyncFunction(f) => self.generate_async_function(f),
            Stmt::Struct(s) if s.is_intrinsic => Ok(()),
            Stmt::Struct(s) => self.generate_struct(s),
            Stmt::Enum(e) => self.generate_enum(e, false),
            Stmt::Expr(e) => {
                self.generate_expression(&e.expr)?;
                self.output.push_str(";\n");
                self.add_line();
                Ok(())
            }
            Stmt::Return(r) => {
                self.output.push_str("return");
                if let Some(ref arg) = r.argument {
                    self.output.push(' ');
                    self.generate_expression(arg)?;
                }
                self.output.push_str(";\n");
                self.add_line();
                Ok(())
            }
            Stmt::If(i) => self.generate_if(i),
            Stmt::While(w) => self.generate_while(w),
            Stmt::For(f) => self.generate_for(f),
            Stmt::Block(b) => self.generate_block(b),
            Stmt::Break(_) => {
                self.output.push_str("break;\n");
                self.add_line();
                Ok(())
            }
            Stmt::Continue(_) => {
                self.output.push_str("continue;\n");
                self.add_line();
                Ok(())
            }
            Stmt::Match(m) => self.generate_match(m),
            Stmt::Switch(s) => self.generate_switch(s),
            Stmt::DoWhile(d) => self.generate_do_while(d),
            Stmt::Import(i) => self.generate_import(i),
            Stmt::Export(e) => self.generate_export(e),
            Stmt::Empty(_) => Ok(()),
            _ => Ok(()),
        }
    }

    /// Rewrite relative import paths to use .js extension for JS runtime compatibility.
    /// Input includes surrounding quotes, e.g. `"./utils"` → `"./utils.js"`.
    pub(crate) fn rewrite_import_source(source: &str) -> String {
        let (open, inner, close) = if source.starts_with('"') && source.ends_with('"') {
            ("\"", &source[1..source.len() - 1], "\"")
        } else if source.starts_with('\'') && source.ends_with('\'') {
            ("'", &source[1..source.len() - 1], "'")
        } else {
            return source.to_string();
        };

        if (inner.starts_with("./") || inner.starts_with("../"))
            && !inner.ends_with(".js")
            && !inner.ends_with(".mjs")
            && !inner.ends_with(".cjs")
            && !inner.ends_with(".json")
        {
            format!("{}{}.js{}", open, inner, close)
        } else {
            source.to_string()
        }
    }

    pub(crate) fn generate_import(
        &mut self,
        import: &argon_ast::ImportStmt,
    ) -> Result<(), CodegenError> {
        if import.is_type_only {
            return Ok(());
        }

        // Check for std:* imports — rewrite to JS platform equivalents
        let raw = import.source.value.trim();
        let spec = raw
            .trim_start_matches('"')
            .trim_end_matches('"')
            .trim_start_matches('\'')
            .trim_end_matches('\'');

        if let Some(module_name) = spec.strip_prefix("std:") {
            return self.generate_std_import(module_name, import);
        }

        if import.specifiers.is_empty() {
            self.output.push_str("import ");
            self.output
                .push_str(&Self::rewrite_import_source(&import.source.value));
            self.output.push_str(";\n");
            self.add_line();
            return Ok(());
        }

        let mut default_import: Option<&str> = None;
        let mut namespace_import: Option<&str> = None;
        let mut named_imports = Vec::new();

        for spec in &import.specifiers {
            match spec {
                argon_ast::ImportSpecifier::Default(d) => {
                    default_import = Some(&d.local.sym);
                }
                argon_ast::ImportSpecifier::Named(n) => {
                    let imported = n.imported.sym.clone();
                    let local = n
                        .local
                        .as_ref()
                        .map(|ident| ident.sym.clone())
                        .unwrap_or_else(|| imported.clone());
                    named_imports.push((imported, local));
                }
                argon_ast::ImportSpecifier::Namespace(n) => {
                    namespace_import = Some(&n.id.sym);
                }
            }
        }

        self.output.push_str("import ");
        let mut needs_comma = false;
        if let Some(default_import) = default_import {
            self.output.push_str(default_import);
            needs_comma = true;
        }
        if let Some(namespace_import) = namespace_import {
            if needs_comma {
                self.output.push_str(", ");
            }
            self.output.push_str("* as ");
            self.output.push_str(namespace_import);
            needs_comma = true;
        }
        if !named_imports.is_empty() {
            if needs_comma {
                self.output.push_str(", ");
            }
            self.output.push_str("{ ");
            for (idx, (imported, local)) in named_imports.iter().enumerate() {
                if idx > 0 {
                    self.output.push_str(", ");
                }
                self.output.push_str(imported);
                if imported != local {
                    self.output.push_str(" as ");
                    self.output.push_str(local);
                }
            }
            self.output.push_str(" }");
        }

        self.output.push_str(" from ");
        self.output
            .push_str(&Self::rewrite_import_source(&import.source.value));
        self.output.push_str(";\n");
        self.add_line();
        Ok(())
    }

    /// Generate JS bindings for a `std:*` import.
    /// Maps each imported symbol to its JS platform equivalent.
    fn generate_std_import(
        &mut self,
        module_name: &str,
        import: &argon_ast::ImportStmt,
    ) -> Result<(), CodegenError> {
        for spec in &import.specifiers {
            match spec {
                argon_ast::ImportSpecifier::Named(n) => {
                    let imported = &n.imported.sym;
                    let local = n
                        .local
                        .as_ref()
                        .map(|ident| ident.sym.as_str())
                        .unwrap_or(imported.as_str());

                    if let Some(js_expr) = std_intrinsics::js_intrinsic(module_name, imported) {
                        // Skip binding when the JS expression is already the same global name
                        if local != js_expr {
                            // std:* imports are always at module scope (no indent)
                            self.output.push_str("const ");
                            self.output.push_str(local);
                            self.output.push_str(" = ");
                            self.output.push_str(js_expr);
                            self.output.push_str(";\n");
                            self.add_line();
                        }
                    } else if let Some(polyfill) =
                        std_intrinsics::js_polyfill(module_name, imported)
                    {
                        // std:* imports are always at module scope (no indent)
                        self.output.push_str("const ");
                        self.output.push_str(local);
                        self.output.push_str(" = ");
                        self.output.push_str(polyfill);
                        self.output.push_str(";\n");
                        self.add_line();
                    }
                }
                argon_ast::ImportSpecifier::Namespace(_)
                | argon_ast::ImportSpecifier::Default(_) => {
                    // Namespace/default imports from std modules are not supported yet
                }
            }
        }
        Ok(())
    }

    pub(crate) fn generate_export(
        &mut self,
        export: &argon_ast::ExportStmt,
    ) -> Result<(), CodegenError> {
        if export.is_type_only {
            return Ok(());
        }

        if let Some(ref decl) = export.declaration {
            match decl.as_ref() {
                Stmt::Function(f) if f.is_intrinsic => {}
                Stmt::Function(f) => {
                    self.output.push_str("export ");
                    self.generate_function(f)?;
                }
                Stmt::AsyncFunction(f) if f.is_intrinsic => {}
                Stmt::AsyncFunction(f) => {
                    self.output.push_str("export ");
                    self.generate_async_function(f)?;
                }
                Stmt::Struct(s) if s.is_intrinsic => {}
                Stmt::Struct(s) => {
                    self.output.push_str("export ");
                    self.generate_struct(s)?;
                }
                Stmt::Variable(v) => {
                    self.output.push_str("export ");
                    self.generate_variable(v)?;
                }
                Stmt::Enum(e) => {
                    self.generate_enum(e, true)?;
                }
                _ => {}
            }
        } else {
            self.output.push_str("export { ");
            for (i, spec) in export.specifiers.iter().enumerate() {
                if i > 0 {
                    self.output.push_str(", ");
                }
                self.output.push_str(&spec.orig.sym);
                if let Some(ref exported) = spec.exported {
                    self.output.push_str(" as ");
                    self.output.push_str(&exported.sym);
                }
            }
            self.output.push_str(" }");
            if let Some(ref source) = export.source {
                self.output.push_str(" from ");
                self.output
                    .push_str(&Self::rewrite_import_source(&source.value));
            }
            self.output.push_str(";\n");
            self.add_line();
        }
        Ok(())
    }

    pub(crate) fn generate_variable(&mut self, v: &VariableStmt) -> Result<(), CodegenError> {
        self.generate_variable_internal(v, true)
    }

    pub(crate) fn generate_variable_internal(
        &mut self,
        v: &VariableStmt,
        add_semicolon: bool,
    ) -> Result<(), CodegenError> {
        let keyword = match v.kind {
            VariableKind::Var => "var",
            VariableKind::Let => "let",
            VariableKind::Const => "const",
        };

        self.output.push_str(keyword);
        self.output.push(' ');

        for (i, decl) in v.declarations.iter().enumerate() {
            if i > 0 {
                self.output.push_str(", ");
            }

            if let Pattern::Identifier(id) = &decl.id {
                self.output.push_str(&id.name.sym);

                if let Some(ref init) = decl.init {
                    self.output.push_str(" = ");
                    self.generate_expression(init)?;
                }
            }
        }
        if add_semicolon {
            self.output.push_str(";\n");
        }
        Ok(())
    }

    pub(crate) fn generate_function(&mut self, f: &FunctionDecl) -> Result<(), CodegenError> {
        self.output.push_str("function ");
        if let Some(ref id) = f.id {
            self.output.push_str(&id.sym);
        }
        self.output.push('(');

        for (i, p) in f.params.iter().enumerate() {
            if i > 0 {
                self.output.push_str(", ");
            }
            if let Pattern::Identifier(id) = &p.pat {
                self.output.push_str(&id.name.sym);
            }
            if let Some(ref default) = p.default {
                self.output.push_str(" = ");
                self.generate_expression(default)?;
            }
        }
        self.output.push_str(") {\n");

        self.indent += 1;
        for stmt in &f.body.statements {
            self.generate_statement(stmt)?;
        }
        self.indent -= 1;

        self.output.push_str("}\n\n");
        Ok(())
    }

    pub(crate) fn generate_struct(&mut self, s: &StructDecl) -> Result<(), CodegenError> {
        // Generate a constructor function for the struct.
        // The parser lowers `Point { x: 1 }` into `new Point({ x: 1 })`,
        // so the constructor expects a single initializer object.
        self.output.push_str("function ");
        self.output.push_str(&s.id.sym);
        self.output.push_str("(init) {\n");

        if let Some(constructor) = &s.constructor {
            // Extract constructor params from init object
            for param in &constructor.params {
                if let Pattern::Identifier(id) = &param.pat {
                    self.output.push_str("    var ");
                    self.output.push_str(&id.name.sym);
                    self.output.push_str(" = init.");
                    self.output.push_str(&id.name.sym);
                    if let Some(ref default) = param.default {
                        self.output.push_str(" ?? ");
                        self.generate_expression(default)?;
                    }
                    self.output.push_str(";\n");
                }
            }
            // Generate constructor body
            for stmt in &constructor.body.statements {
                self.generate_statement(stmt)?;
            }
        } else {
            // No constructor - assign fields from init
            for field in &s.fields {
                self.output.push_str("    this.");
                self.output.push_str(&field.id.sym);
                self.output.push_str(" = init.");
                self.output.push_str(&field.id.sym);
                self.output.push_str(";\n");
            }
        }

        // Collect struct's own method names for priority checking
        let struct_method_names: Vec<String> = s
            .methods
            .iter()
            .filter_map(|m| match &m.key {
                Expr::Identifier(id) => Some(id.sym.clone()),
                _ => None,
            })
            .collect();

        // Generate methods from embodied skills (concrete methods only)
        for skill_name in &s.embodies {
            if let Some(skill_decl) = self.skill_defs.get(&skill_name.sym).cloned() {
                for item in &skill_decl.items {
                    if let SkillItem::ConcreteMethod(method) = item {
                        let method_name = match &method.key {
                            Expr::Identifier(id) => id.sym.clone(),
                            _ => continue,
                        };
                        // Skip if struct provides its own implementation
                        if struct_method_names.contains(&method_name) {
                            continue;
                        }
                        self.output.push_str("this.");
                        self.generate_expression(&method.key)?;
                        self.output.push_str(" = function(");
                        for (i, p) in method.value.params.iter().enumerate() {
                            if i > 0 {
                                self.output.push_str(", ");
                            }
                            if let Pattern::Identifier(id) = &p.pat {
                                self.output.push_str(&id.name.sym);
                            }
                            if let Some(ref default) = p.default {
                                self.output.push_str(" = ");
                                self.generate_expression(default)?;
                            }
                        }
                        self.output.push_str(") {\n");
                        for stmt in &method.value.body.statements {
                            self.generate_statement(stmt)?;
                        }
                        self.output.push_str("}.bind(this);\n");
                    }
                }
            }
        }

        // Generate struct's own methods
        for method in &s.methods {
            self.output.push_str("this.");
            self.generate_expression(&method.key)?;
            self.output.push_str(" = function(");
            for (i, p) in method.value.params.iter().enumerate() {
                if i > 0 {
                    self.output.push_str(", ");
                }
                if let Pattern::Identifier(id) = &p.pat {
                    self.output.push_str(&id.name.sym);
                }
                if let Some(ref default) = p.default {
                    self.output.push_str(" = ");
                    self.generate_expression(default)?;
                }
            }
            self.output.push_str(") {\n");
            for stmt in &method.value.body.statements {
                self.generate_statement(stmt)?;
            }
            self.output.push_str("};\n");
        }

        self.output.push_str("}\n\n");
        Ok(())
    }

    fn generate_if(&mut self, i: &IfStmt) -> Result<(), CodegenError> {
        self.output.push_str("if (");
        self.generate_expression(&i.condition)?;
        self.output.push_str(") {\n");

        self.indent += 1;
        self.generate_statement(&i.consequent)?;
        self.indent -= 1;

        if let Some(ref alt) = i.alternate {
            self.output.push_str("} else {\n");
            self.indent += 1;
            self.generate_statement(alt)?;
            self.indent -= 1;
        }

        self.output.push_str("}\n");
        Ok(())
    }

    fn generate_while(&mut self, w: &WhileStmt) -> Result<(), CodegenError> {
        self.output.push_str("while (");
        self.generate_expression(&w.condition)?;
        self.output.push_str(") {\n");

        self.indent += 1;
        self.generate_statement(&w.body)?;
        self.indent -= 1;

        self.output.push_str("}\n");
        Ok(())
    }

    fn generate_for(&mut self, f: &ForStmt) -> Result<(), CodegenError> {
        self.output.push_str("for (");

        if let Some(ref init) = f.init {
            match init {
                ForInit::Variable(v) => self.generate_variable_internal(v, false)?,
                ForInit::Expr(e) => self.generate_expression(e)?,
            }
        }

        self.output.push(';');

        if let Some(ref test) = f.test {
            self.output.push(' ');
            self.generate_expression(test)?;
        }

        if let Some(update) = &f.update {
            self.output.push_str("; ");
            self.generate_expression(update)?;
        }

        self.output.push_str(") ");

        // Generate body
        match &*f.body {
            Stmt::Block(b) => {
                self.output.push_str("{\n");
                self.indent += 1;
                for stmt in &b.statements {
                    self.generate_statement(stmt)?;
                }
                self.indent -= 1;
                self.output.push_str("}\n");
            }
            _ => {
                self.output.push_str("{\n");
                self.indent += 1;
                self.generate_statement(&f.body)?;
                self.indent -= 1;
                self.output.push_str("}\n");
            }
        }
        Ok(())
    }

    fn generate_block(&mut self, b: &BlockStmt) -> Result<(), CodegenError> {
        self.output.push_str("{\n");
        self.indent += 1;

        for stmt in &b.statements {
            self.generate_statement(stmt)?;
        }

        self.indent -= 1;
        self.output.push_str("}\n");
        Ok(())
    }

    fn generate_match(&mut self, m: &MatchStmt) -> Result<(), CodegenError> {
        let has_result_patterns = m
            .cases
            .iter()
            .any(|case| matches!(case.pattern, MatchPattern::Result(_)));

        if !has_result_patterns {
            self.output.push_str("switch (");
            self.generate_expression(&m.discriminant)?;
            self.output.push_str(") {\n");

            for case in &m.cases {
                self.output.push_str("case ");
                match &case.pattern {
                    MatchPattern::Expr(pattern) => self.generate_expression(pattern)?,
                    MatchPattern::Result(_) => unreachable!("result patterns handled above"),
                }
                self.output.push_str(":\n");

                self.indent += 1;
                self.generate_statement(case.consequent.as_ref())?;
                self.indent -= 1;
            }

            self.output.push_str("}\n");
            return Ok(());
        }

        self.loop_label_counter += 1;
        let discr_name = format!("__argon_match_{}", self.loop_label_counter);
        self.output.push_str("const ");
        self.output.push_str(&discr_name);
        self.output.push_str(" = ");
        self.generate_expression(&m.discriminant)?;
        self.output.push_str(";\n");

        for (index, case) in m.cases.iter().enumerate() {
            if index == 0 {
                self.output.push_str("if (");
            } else {
                self.output.push_str("else if (");
            }

            match &case.pattern {
                MatchPattern::Expr(pattern) => {
                    self.output.push_str(&discr_name);
                    self.output.push_str(" === ");
                    self.generate_expression(pattern)?;
                }
                MatchPattern::Result(pattern) => match pattern.kind {
                    ResultPatternKind::Ok => {
                        self.output.push_str(&format!(
                            "{} && ({}.__tag === \"Ok\" || {}.isOk === true)",
                            discr_name, discr_name, discr_name
                        ));
                    }
                    ResultPatternKind::Err => {
                        self.output.push_str(&format!(
                            "{} && ({}.__tag === \"Err\" || {}.isErr === true)",
                            discr_name, discr_name, discr_name
                        ));
                    }
                },
            }

            self.output.push_str(") {\n");
            self.indent += 1;

            if let MatchPattern::Result(pattern) = &case.pattern {
                self.output.push_str("const ");
                self.output.push_str(&pattern.binding.sym);
                self.output.push_str(" = ");
                self.output.push_str(&discr_name);
                self.output.push_str(match pattern.kind {
                    ResultPatternKind::Ok => ".value;\n",
                    ResultPatternKind::Err => ".error;\n",
                });
            }

            self.generate_statement(case.consequent.as_ref())?;
            self.indent -= 1;
            self.output.push_str("}\n");
        }

        Ok(())
    }

    pub(crate) fn generate_async_function(&mut self, f: &FunctionDecl) -> Result<(), CodegenError> {
        self.output.push_str("async function ");
        if let Some(ref id) = f.id {
            self.output.push_str(&id.sym);
        }
        self.output.push('(');

        for (i, p) in f.params.iter().enumerate() {
            if i > 0 {
                self.output.push_str(", ");
            }
            if let Pattern::Identifier(id) = &p.pat {
                self.output.push_str(&id.name.sym);
            }
            if let Some(ref default) = p.default {
                self.output.push_str(" = ");
                self.generate_expression(default)?;
            }
        }
        self.output.push_str(") {\n");

        self.indent += 1;
        for stmt in &f.body.statements {
            self.generate_statement(stmt)?;
        }
        self.indent -= 1;

        self.output.push_str("}\n\n");
        Ok(())
    }

    pub(crate) fn generate_enum(&mut self, e: &EnumDecl, export: bool) -> Result<(), CodegenError> {
        if export {
            self.output.push_str("export ");
        }
        self.output.push_str("const ");
        self.output.push_str(&e.id.sym);
        self.output.push_str(" = { ");

        let mut next_numeric = Some(0.0);
        for (idx, member) in e.members.iter().enumerate() {
            if idx > 0 {
                self.output.push_str(", ");
            }
            self.output.push_str(&member.id.sym);
            self.output.push_str(": ");

            if let Some(init) = &member.init {
                self.generate_expression(init)?;
                next_numeric = match init {
                    Expr::Literal(Literal::Number(n)) => Some(n.value + 1.0),
                    _ => None,
                };
            } else if let Some(current) = next_numeric {
                if current.fract() == 0.0 {
                    self.output.push_str(&(current as i64).to_string());
                } else {
                    self.output.push_str(&current.to_string());
                }
                next_numeric = Some(current + 1.0);
            } else {
                return Err(CodegenError::Unsupported(format!(
                    "enum member '{}' requires an explicit initializer after a non-numeric member",
                    member.id.sym
                )));
            }
        }

        self.output.push_str(" };\n");
        self.add_line();
        Ok(())
    }

    fn generate_switch(&mut self, s: &SwitchStmt) -> Result<(), CodegenError> {
        self.output.push_str("switch (");
        self.generate_expression(&s.discriminant)?;
        self.output.push_str(") {\n");

        for case in &s.cases {
            if let Some(ref test) = case.test {
                self.output.push_str("case ");
                self.generate_expression(test)?;
                self.output.push_str(":\n");
            } else {
                self.output.push_str("default:\n");
            }

            self.indent += 1;
            for stmt in &case.consequent {
                self.generate_statement(stmt)?;
            }
            self.indent -= 1;
        }

        self.output.push_str("}\n");
        Ok(())
    }

    fn generate_do_while(&mut self, d: &DoWhileStmt) -> Result<(), CodegenError> {
        self.output.push_str("do {\n");
        self.indent += 1;
        self.generate_statement(&d.body)?;
        self.indent -= 1;
        self.output.push_str("} while (");
        self.generate_expression(&d.condition)?;
        self.output.push_str(");\n");
        Ok(())
    }

    #[allow(dead_code)]
    pub(crate) fn indent(&mut self) {
        for _ in 0..self.indent {
            self.output.push_str("    ");
        }
    }
}
