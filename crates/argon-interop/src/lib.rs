//! Argon - JS Interop
//! Handles safe interoperability between Argon and JavaScript

use argon_ast::*;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub enum JsType {
    Any,
    String,
    Number,
    Boolean,
    Object,
    Array,
    Function,
    Promise,
    Symbol,
    BigInt,
}

#[derive(Debug, Clone)]
pub struct JsImport {
    pub module: String,
    pub name: String,
    pub local_name: String,
    pub js_type: JsType,
}

#[derive(Debug, Clone)]
pub struct JsExport {
    pub name: String,
    pub ty: Type,
    pub annotations: Vec<ExportAnnotation>,
}

#[derive(Debug, Clone)]
pub enum ExportAnnotation {
    Export,
    JsInterop,
}

pub struct InteropGenerator {
    imports: Vec<JsImport>,
    exports: Vec<JsExport>,
    type_mappings: HashMap<String, String>,
    shared_wrappers: HashMap<String, String>,
}

impl InteropGenerator {
    pub fn new() -> Self {
        let mut gen = Self {
            imports: Vec::new(),
            exports: Vec::new(),
            type_mappings: HashMap::new(),
            shared_wrappers: HashMap::new(),
        };
        gen.init_builtin_mappings();
        gen
    }

    fn init_builtin_mappings(&mut self) {
        self.type_mappings
            .insert("string".to_string(), "String".to_string());
        self.type_mappings
            .insert("number".to_string(), "Number".to_string());
        self.type_mappings
            .insert("boolean".to_string(), "Boolean".to_string());
        self.type_mappings
            .insert("any".to_string(), "any".to_string());
        self.type_mappings
            .insert("object".to_string(), "Object".to_string());
        self.type_mappings
            .insert("unknown".to_string(), "any".to_string());

        self.shared_wrappers
            .insert("string".to_string(), "new SharedString(#)".to_string());
        self.shared_wrappers
            .insert("number".to_string(), "new SharedNumber(#)".to_string());
        self.shared_wrappers
            .insert("boolean".to_string(), "new SharedBoolean(#)".to_string());
        self.shared_wrappers
            .insert("object".to_string(), "new SharedObject(#)".to_string());
    }

    pub fn process_import(&mut self, import: &ImportStmt) -> Result<JsImport, InteropError> {
        let module = import.source.value.clone();

        let mut js_import = JsImport {
            module,
            name: String::new(),
            local_name: String::new(),
            js_type: JsType::Any,
        };

        for specifier in &import.specifiers {
            match specifier {
                ImportSpecifier::Default(d) => {
                    js_import.name = "default".to_string();
                    js_import.local_name = d.local.sym.clone();
                }
                ImportSpecifier::Named(n) => {
                    js_import.name = n.imported.sym.clone();
                    js_import.local_name = n
                        .local
                        .as_ref()
                        .map(|l| l.sym.clone())
                        .unwrap_or_else(|| n.imported.sym.clone());
                }
                ImportSpecifier::Namespace(n) => {
                    js_import.name = "*".to_string();
                    js_import.local_name = n.id.sym.clone();
                }
            }
        }

        self.imports.push(js_import.clone());
        Ok(js_import)
    }

    pub fn process_export(
        &mut self,
        export: &ExportStmt,
        stmt: &Stmt,
    ) -> Result<JsExport, InteropError> {
        let (name, ty) = match stmt {
            Stmt::Function(f) => {
                let func_name = f.id.as_ref().map(|i| i.sym.clone()).unwrap_or_default();
                let ty = Type::Function(FunctionType {
                    type_params: vec![],
                    params: f
                        .params
                        .iter()
                        .map(|p| FunctionTypeParam {
                            name: None,
                            ty: *p
                                .ty
                                .clone()
                                .unwrap_or(Box::new(Type::Any(AnyType { span: 0..0 }))),
                            optional: false,
                        })
                        .collect(),
                    return_type: f
                        .return_type
                        .clone()
                        .unwrap_or(Box::new(Type::Void(VoidType { span: 0..0 }))),
                    span: 0..0,
                });
                (func_name, ty)
            }
            Stmt::Variable(v) => {
                if let Some(decl) = v.declarations.first() {
                    if let Pattern::Identifier(id) = &decl.id {
                        let ty = *id
                            .type_annotation
                            .clone()
                            .unwrap_or(Box::new(Type::Any(AnyType { span: 0..0 })));
                        (id.name.sym.clone(), ty)
                    } else {
                        return Err(InteropError::InvalidExport);
                    }
                } else {
                    return Err(InteropError::InvalidExport);
                }
            }
            _ => return Err(InteropError::InvalidExport),
        };

        let mut annotations = Vec::new();
        if export.declaration.is_some() {
            annotations.push(ExportAnnotation::Export);
        }

        let js_export = JsExport {
            name,
            ty: ty.clone(),
            annotations,
        };

        self.exports.push(js_export.clone());
        Ok(js_export)
    }

    pub fn generate_wrapper(&self, import: &JsImport) -> String {
        let wrapper_name = format!("{}_wrapper", import.local_name);

        format!(
            r#"function {}(...args) {{
    return Argon.Shared.wrap({}, ...args);
}}"#,
            wrapper_name, import.local_name
        )
    }

    pub fn generate_import_wrapper(&self, import: &JsImport) -> String {
        let wrapper = self.shared_wrappers.get(&format!("{:?}", import.js_type));

        if let Some(w) = wrapper {
            return w.replace("#", &import.local_name);
        }

        format!("Argon.Shared.wrap({})", import.local_name)
    }

    pub fn generate_export_wrapper(&self, export: &JsExport) -> String {
        format!("Argon.export({})", export.name)
    }

    pub fn map_argon_to_js(&self, ty: &Type) -> String {
        match ty {
            Type::Primitive(p) => match p {
                PrimitiveType::String => "string".to_string(),
                PrimitiveType::Number => "number".to_string(),
                PrimitiveType::Boolean => "boolean".to_string(),
                PrimitiveType::Void => "undefined".to_string(),
                PrimitiveType::Any => "any".to_string(),
                _ => "any".to_string(),
            },
            Type::Reference(r) => {
                if let TypeName::Ident(id) = &r.name {
                    self.type_mappings
                        .get(&id.sym)
                        .cloned()
                        .unwrap_or_else(|| "any".to_string())
                } else {
                    "any".to_string()
                }
            }
            Type::Array(_) => "Array".to_string(),
            Type::Function(_) => "Function".to_string(),
            Type::Object(_) => "Object".to_string(),
            _ => "any".to_string(),
        }
    }

    pub fn map_js_to_argon(&self, js_type: &str) -> String {
        match js_type {
            "string" => "string".to_string(),
            "number" => "number".to_string(),
            "boolean" => "boolean".to_string(),
            "object" => "object".to_string(),
            "function" => "Function".to_string(),
            "symbol" => "Symbol".to_string(),
            "bigint" => "bigint".to_string(),
            "undefined" => "void".to_string(),
            _ => "any".to_string(),
        }
    }

    pub fn generate_declaration_file(&self) -> String {
        let mut output = String::from("// Generated type declarations\n\n");

        for export in &self.exports {
            output.push_str(&format!(
                "export function {}: {};\n",
                export.name,
                self.map_argon_to_js(&export.ty)
            ));
        }

        output
    }
}

#[derive(Debug)]
pub enum InteropError {
    InvalidImport,
    InvalidExport,
    TypeMismatch(String),
}

impl std::fmt::Display for InteropError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InteropError::InvalidImport => write!(f, "Invalid import"),
            InteropError::InvalidExport => write!(f, "Invalid export"),
            InteropError::TypeMismatch(msg) => write!(f, "Type mismatch: {}", msg),
        }
    }
}

impl std::error::Error for InteropError {}

#[cfg(test)]
mod interop_tests {
    use super::*;
    use argon_parser::parse;

    #[test]
    fn processes_named_import_alias() {
        // Assign
        let source = r#"import { useState as state } from "react";"#;
        let ast = parse(source).expect("parse should succeed");
        let import = match &ast.statements[0] {
            Stmt::Import(i) => i,
            _ => panic!("expected import statement"),
        };
        let mut generator = InteropGenerator::new();

        // Act
        let js_import = generator
            .process_import(import)
            .expect("import processing should succeed");

        // Assert
        assert!(js_import.module.contains("react"));
        assert_eq!(js_import.name, "useState");
        assert_eq!(js_import.local_name, "state");
    }

    #[test]
    fn processes_exported_function_declaration() {
        // Assign
        let source = r#"
            export function greet(name: string): string {
                return name;
            }
        "#;
        let ast = parse(source).expect("parse should succeed");
        let export = match &ast.statements[0] {
            Stmt::Export(e) => e,
            _ => panic!("expected export statement"),
        };
        let decl = export
            .declaration
            .as_deref()
            .expect("export should contain declaration");
        let mut generator = InteropGenerator::new();

        // Act
        let js_export = generator
            .process_export(export, decl)
            .expect("export processing should succeed");

        // Assert
        assert_eq!(js_export.name, "greet");
        assert_eq!(generator.map_argon_to_js(&js_export.ty), "Function");
    }

    #[test]
    fn generates_declaration_file_from_exports() {
        // Assign
        let source = r#"export function ping(): string { return "ok"; }"#;
        let ast = parse(source).expect("parse should succeed");
        let export = match &ast.statements[0] {
            Stmt::Export(e) => e,
            _ => panic!("expected export statement"),
        };
        let decl = export
            .declaration
            .as_deref()
            .expect("export should contain declaration");
        let mut generator = InteropGenerator::new();
        generator
            .process_export(export, decl)
            .expect("export processing should succeed");

        // Act
        let dts = generator.generate_declaration_file();

        // Assert
        assert!(dts.contains("Generated type declarations"));
        assert!(dts.contains("export function ping: Function;"));
    }
}
