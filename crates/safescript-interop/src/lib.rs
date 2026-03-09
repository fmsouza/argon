//! SafeScript - JS Interop

pub struct Interop;

impl Interop {
    pub fn new() -> Self {
        Self
    }

    pub fn wrap_js_value(&self, ty: &str) -> String {
        format!("Shared({})", ty)
    }

    pub fn unwrap_js_value(&self, ty: &str) -> String {
        format!("unwrap({})", ty)
    }
}
