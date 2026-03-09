//! SafeScript - Standard library

pub struct StdLib;

impl StdLib {
    pub fn new() -> Self {
        Self
    }

    pub fn get_primitives() -> Vec<(&'static str, &'static str)> {
        vec![
            ("i8", "number"),
            ("i16", "number"),
            ("i32", "number"),
            ("i64", "BigInt"),
            ("u8", "number"),
            ("u16", "number"),
            ("u32", "number"),
            ("u64", "BigInt"),
            ("f32", "number"),
            ("f64", "number"),
            ("bool", "boolean"),
            ("string", "string"),
        ]
    }

    pub fn get_runtime() -> &'static str {
        r#"
const __safescript = {
    Vec: class Vec {
        constructor() {
            this.data = [];
        }
        push(...items) {
            this.data.push(...items);
        }
        get(i) {
            return this.data[i];
        }
        length() {
            return this.data.length;
        }
    },
    Option: {
        Some: class Some {
            constructor(value) { this.value = value; }
            isSome() { return true; }
            isNone() { return false; }
            unwrap() { return this.value; }
        },
        None: class None {
            isSome() { return false; }
            isNone() { return true; }
            unwrap() { throw new Error("unwrap on None"); }
        }
    },
    Shared: class Shared {
        constructor(value) {
            this.value = value;
            this.refs = 1;
        }
        clone() {
            this.refs++;
            return this;
        }
        drop() {
            this.refs--;
            if (this.refs <= 0) {
                // cleanup
            }
        }
    },
    Result: {
        Ok: class Ok {
            constructor(value) { this.value = value; }
            isOk() { return true; }
            isErr() { return false; }
            unwrap() { return this.value; }
        },
        Err: class Err {
            constructor(error) { this.error = error; }
            isOk() { return false; }
            isErr() { return true; }
            unwrap() { throw this.error; }
        }
    }
};
"#
    }
}
