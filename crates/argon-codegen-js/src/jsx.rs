use argon_ast::*;

use super::*;

impl JsCodegen {
    pub(crate) fn generate_jsx_element(
        &mut self,
        elem: &JsxElement,
    ) -> Result<(), CodegenError> {
        self.output.push_str("React.createElement(");

        // Generate element name
        self.generate_jsx_element_name(&elem.opening.name)?;

        // Build attributes object
        if !elem.opening.attributes.is_empty() {
            self.output.push_str(", { ");
            for (i, attr) in elem.opening.attributes.iter().enumerate() {
                if i > 0 {
                    self.output.push_str(", ");
                }
                self.generate_jsx_attribute(attr)?;
            }
            self.output.push_str(" }");
        } else {
            self.output.push_str(", null");
        }

        // Build children
        if elem.children.is_empty() {
            self.output.push_str(", null");
        } else {
            self.output.push_str(", ");
            for (i, child) in elem.children.iter().enumerate() {
                if i > 0 {
                    self.output.push_str(", ");
                }
                self.generate_jsx_child(child)?;
            }
        }

        self.output.push(')');
        Ok(())
    }

    pub(crate) fn generate_jsx_fragment(
        &mut self,
        frag: &JsxFragment,
    ) -> Result<(), CodegenError> {
        self.output
            .push_str("React.createElement(React.Fragment, null, ");
        for (i, child) in frag.children.iter().enumerate() {
            if i > 0 {
                self.output.push_str(", ");
            }
            self.generate_jsx_child(child)?;
        }
        self.output.push(')');
        Ok(())
    }

    fn generate_jsx_element_name(&mut self, name: &JsxElementName) -> Result<(), CodegenError> {
        match name {
            JsxElementName::Identifier(id) => {
                self.output.push_str(&format!("\"{}\"", id.sym));
            }
            JsxElementName::Namespaced(ns) => {
                self.output
                    .push_str(&format!("\"{}:{}\"", ns.namespace.sym, ns.name.sym));
            }
            JsxElementName::Member(m) => {
                self.generate_jsx_member_name(m)?;
            }
        }
        Ok(())
    }

    fn generate_jsx_member_name(&mut self, name: &JsxElementName) -> Result<(), CodegenError> {
        match name {
            JsxElementName::Identifier(id) => {
                self.output.push_str(&format!("\"{}\"", id.sym));
            }
            JsxElementName::Member(m) => {
                self.generate_jsx_member_name(m)?;
                if let JsxElementName::Identifier(id) = m.as_ref() {
                    self.output.push_str(&format!(".{}", id.sym));
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn generate_jsx_attribute(&mut self, attr: &JsxAttribute) -> Result<(), CodegenError> {
        let name = match &attr.name {
            JsxAttributeName::Identifier(id) => &id.sym,
            JsxAttributeName::Namespaced(ns) => &format!("{}:{}", ns.namespace.sym, ns.name.sym),
        };

        if let Some(value) = &attr.value {
            match value {
                JsxAttributeValue::String(s) => {
                    self.output.push_str(&format!("{}: \"{}\"", name, s.value));
                }
                JsxAttributeValue::Expression(e) => {
                    self.output.push_str(name);
                    self.output.push_str(": (");
                    self.generate_expression(e)?;
                    self.output.push(')');
                }
                JsxAttributeValue::Element(_) | JsxAttributeValue::Fragment(_) => {
                    self.output.push_str(name);
                    self.output.push_str(": ");
                    // Handle nested elements
                }
                JsxAttributeValue::Span(_) => {}
            }
        } else {
            // Boolean attribute
            self.output.push_str(&format!("{}: true", name));
        }
        Ok(())
    }

    fn generate_jsx_child(&mut self, child: &JsxChild) -> Result<(), CodegenError> {
        match child {
            JsxChild::Text(t) => {
                self.output
                    .push_str(&format!("\"{}\"", t.value.replace("\"", "\\\"")));
            }
            JsxChild::Expression(e) => {
                self.generate_expression(e)?;
            }
            JsxChild::Element(e) => {
                self.output.push_str("React.createElement(");
                self.generate_jsx_element(e)?;
                self.output.push(')');
            }
            JsxChild::Fragment(f) => {
                self.output.push_str("React.Fragment(null, [");
                for (i, child) in f.children.iter().enumerate() {
                    if i > 0 {
                        self.output.push_str(", ");
                    }
                    self.generate_jsx_child(child)?;
                }
                self.output.push_str("])");
            }
            JsxChild::Spread(s) => {
                self.output.push_str("...");
                self.generate_expression(&s.expression)?;
            }
        }
        Ok(())
    }
}
