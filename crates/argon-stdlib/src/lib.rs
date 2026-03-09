//! Argon - Standard library

use std::collections::HashMap;

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
(function(global) {
    'use strict';

    // Vec<T> - Dynamic array with ownership semantics
    function Vec(capacity) {
        this.data = new Array(capacity || 0);
        this.len = 0;
    }

    Vec.prototype.push = function() {
        for (let i = 0; i < arguments.length; i++) {
            this.data[this.len++] = arguments[i];
        }
        return this.len;
    };

    Vec.prototype.pop = function() {
        if (this.len === 0) return undefined;
        return this.data[--this.len];
    };

    Vec.prototype.get = function(index) {
        if (index < 0 || index >= this.len) {
            throw new Error('Vec index out of bounds');
        }
        return this.data[index];
    };

    Vec.prototype.set = function(index, value) {
        if (index < 0 || index >= this.len) {
            throw new Error('Vec index out of bounds');
        }
        this.data[index] = value;
    };

    Object.defineProperty(Vec.prototype, 'len', {
        get: function() { return this.len; },
        configurable: false
    });

    Vec.prototype.isEmpty = function() {
        return this.len === 0;
    };

    Vec.prototype.clear = function() {
        this.len = 0;
        this.data = [];
    };

    Vec.prototype[Symbol.iterator] = function*() {
        for (let i = 0; i < this.len; i++) {
            yield this.data[i];
        }
    };

    Vec.prototype.map = function(fn) {
        const result = new Vec();
        for (let i = 0; i < this.len; i++) {
            result.push(fn(this.data[i], i));
        }
        return result;
    };

    Vec.prototype.filter = function(fn) {
        const result = new Vec();
        for (let i = 0; i < this.len; i++) {
            if (fn(this.data[i], i)) {
                result.push(this.data[i]);
            }
        }
        return result;
    };

    Vec.prototype.reduce = function(fn, initial) {
        let acc = initial;
        for (let i = 0; i < this.len; i++) {
            acc = fn(acc, this.data[i], i);
        }
        return acc;
    };

    // Option<T> - Optional value type
    function Some(value) {
        this.value = value;
    }

    Some.prototype.isSome = function() { return true; };
    Some.prototype.isNone = function() { return false; };
    Some.prototype.unwrap = function() { return this.value; };
    Some.prototype.unwrapOr = function() { return this.value; };
    Some.prototype.map = function(fn) { return new Some(fn(this.value)); };

    function None() {}

    None.prototype.isSome = function() { return false; };
    None.prototype.isNone = function() { return true; };
    None.prototype.unwrap = function() { throw new Error('Called unwrap on None'); };
    None.prototype.unwrapOr = function(default_) { return default_; };
    None.prototype.map = function() { return new None(); };

    // Result<T, E> - Result type for error handling
    function Ok(value) {
        this.value = value;
    }

    Ok.prototype.isOk = function() { return true; };
    Ok.prototype.isErr = function() { return false; };
    Ok.prototype.unwrap = function() { return this.value; };
    Ok.prototype.unwrapErr = function() { throw new Error('Called unwrapErr on Ok'); };
    Ok.prototype.map = function(fn) { return new Ok(fn(this.value)); };
    Ok.prototype.mapErr = function() { return this; };

    function Err(error) {
        this.error = error;
    }

    Err.prototype.isOk = function() { return false; };
    Err.prototype.isErr = function() { return true; };
    Err.prototype.unwrap = function() { throw this.error; };
    Err.prototype.unwrapErr = function() { return this.error; };
    Err.prototype.map = function() { return this; };
    Err.prototype.mapErr = function(fn) { return new Err(fn(this.error)); };

    // Shared<T> - Shared ownership (Arc-like)
    function Shared(value) {
        this.value = value;
        this.refs = 1;
    }

    Shared.prototype.clone = function() {
        this.refs++;
        return this;
    };

    Shared.prototype.drop = function() {
        this.refs--;
        return this.refs <= 0;
    };

    Shared.prototype.get = function() {
        return this.value;
    };

    Shared.prototype.set = function(value) {
        this.value = value;
    };

    Shared.wrap = function(value) {
        return new Shared(value);
    };

    Shared.unwrap = function(shared) {
        if (shared instanceof Shared) {
            return shared.get();
        }
        return shared;
    };

    // String - Owned string type
    function String(str) {
        this.data = String(str);
    }

    Object.defineProperty(String.prototype, 'len', {
        get: function() { return this.data.length; },
        configurable: false
    });

    String.prototype.isEmpty = function() {
        return this.data.length === 0;
    };

    String.prototype.charAt = function(index) {
        return this.data.charAt(index);
    };

    String.prototype.concat = function() {
        return new String(this.data.concat(...arguments));
    };

    String.prototype.contains = function(substr) {
        return this.data.includes(String(substr));
    };

    String.prototype.endsWith = function(suffix) {
        return this.data.endsWith(String(suffix));
    };

    String.prototype.startsWith = function(prefix) {
        return this.data.startsWith(String(prefix));
    };

    String.prototype.indexOf = function(substr) {
        return this.data.indexOf(String(substr));
    };

    String.prototype.slice = function(start, end) {
        return new String(this.data.slice(start, end));
    };

    String.prototype.split = function(separator) {
        return this.data.split(String(separator)).map(s => new String(s));
    };

    String.prototype.toUpperCase = function() {
        return new String(this.data.toUpperCase());
    };

    String.prototype.toLowerCase = function() {
        return new String(this.data.toLowerCase());
    };

    String.prototype.trim = function() {
        return new String(this.data.trim());
    };

    String.prototype.replace = function(search, replace) {
        return new String(this.data.replace(String(search), String(replace)));
    };

    // Map<K, V> - Key-value store
    function Map() {
        this.data = new Map();
    }

    Map.prototype.set = function(key, value) {
        this.data.set(key, value);
        return this;
    };

    Map.prototype.get = function(key) {
        return this.data.get(key);
    };

    Map.prototype.has = function(key) {
        return this.data.has(key);
    };

    Map.prototype.delete = function(key) {
        return this.data.delete(key);
    };

    Object.defineProperty(Map.prototype, 'len', {
        get: function() { return this.data.size; },
        configurable: false
    });

    Map.prototype.keys = function() {
        return Array.from(this.data.keys());
    };

    Map.prototype.values = function() {
        return Array.from(this.data.values());
    };

    Map.prototype.entries = function() {
        return Array.from(this.data.entries());
    };

    // Set<T> - Unique values collection
    function Set() {
        this.data = new Set();
    }

    Set.prototype.add = function(value) {
        this.data.add(value);
        return this;
    };

    Set.prototype.has = function(value) {
        return this.data.has(value);
    };

    Set.prototype.delete = function(value) {
        return this.data.delete(value);
    };

    Object.defineProperty(Set.prototype, 'len', {
        get: function() { return this.data.size; },
        configurable: false
    });

    Set.prototype.values = function() {
        return Array.from(this.data.values());
    };

    // Test utilities
    const TestRunner = {
        passed: 0,
        failed: 0,
        skipped: 0,
        currentTest: null,
        
        assert: function(condition, message) {
            if (!condition) {
                throw new Error(message || 'Assertion failed');
            }
        },
        
        assertEq: function(actual, expected, message) {
            if (actual !== expected) {
                throw new Error(message || ('Expected ' + expected + ' but got ' + actual));
            }
        },
        
        assertNe: function(actual, expected, message) {
            if (actual === expected) {
                throw new Error(message || ('Expected ' + actual + ' to not equal ' + expected));
            }
        },
        
        assertTrue: function(actual, message) {
            if (actual !== true) {
                throw new Error(message || ('Expected true but got ' + actual));
            }
        },
        
        assertFalse: function(actual, message) {
            if (actual !== false) {
                throw new Error(message || ('Expected false but got ' + actual));
            }
        },
        
        assertNull: function(actual, message) {
            if (actual !== null) {
                throw new Error(message || ('Expected null but got ' + actual));
            }
        },
        
        assertNotNull: function(actual, message) {
            if (actual === null || actual === undefined) {
                throw new Error(message || ('Expected not null but got ' + actual));
            }
        },
        
        assertThrows: function(fn, message) {
            try {
                fn();
                throw new Error(message || 'Expected function to throw');
            } catch (e) {
                if (e.message === message) {
                    throw e;
                }
            }
        },
        
        describe: function(name, fn) {
            console.log('  ' + name);
            try {
                fn();
            } catch (e) {
                console.log('    ERROR: ' + e.message);
                this.failed++;
            }
        },
        
        it: function(name, fn) {
            console.log('    - ' + name);
            this.currentTest = name;
            try {
                fn();
                this.passed++;
            } catch (e) {
                console.log('      FAILED: ' + e.message);
                this.failed++;
            }
        },
        
        skip: function(name, fn) {
            console.log('    - ' + name + ' (SKIPPED)');
            this.skipped++;
        },
        
        test: function(name, fn) {
            this.it(name, fn);
        },
        
        expect: function(value) {
            const self = this;
            return {
                toBe: function(expected) {
                    self.assertEq(value, expected);
                },
                toEqual: function(expected) {
                    self.assertEq(JSON.stringify(value), JSON.stringify(expected));
                },
                toBeTruthy: function() {
                    self.assert(!!value, 'Expected truthy value');
                },
                toBeFalsy: function() {
                    self.assert(!value, 'Expected falsy value');
                },
                toBeNull: function() {
                    self.assertNull(value);
                },
                toBeDefined: function() {
                    self.assert(value !== undefined, 'Expected defined value');
                },
                toBeUndefined: function() {
                    self.assert(value === undefined, 'Expected undefined');
                },
                toContain: function(item) {
                    self.assert(value.includes(item), 'Expected to contain ' + item);
                },
                toThrow: function() {
                    self.assertThrows(function() { value(); });
                },
                not: {
                    toBe: function(expected) {
                        self.assertNe(value, expected);
                    },
                    toBeTruthy: function() {
                        self.assert(!value, 'Expected falsy value');
                    },
                    toBeNull: function() {
                        self.assert(value !== null, 'Expected not null');
                    }
                }
            };
        },
        
        summary: function() {
            console.log('\nTest Summary:');
            console.log('  Passed: ' + this.passed);
            console.log('  Failed: ' + this.failed);
            console.log('  Skipped: ' + this.skipped);
            return { passed: this.passed, failed: this.failed, skipped: this.skipped };
        },
        
        reset: function() {
            this.passed = 0;
            this.failed = 0;
            this.skipped = 0;
            this.currentTest = null;
        }
    };
    
    global.test = TestRunner.test.bind(TestRunner);
    global.it = TestRunner.it.bind(TestRunner);
    global.describe = TestRunner.describe.bind(TestRunner);
    global.expect = TestRunner.expect.bind(TestRunner);
    global.assert = TestRunner.assert;
    global.assertEq = TestRunner.assertEq;
    global.assertNe = TestRunner.assertNe;
    global.assertTrue = TestRunner.assertTrue;
    global.assertFalse = TestRunner.assertFalse;
    global.assertNull = TestRunner.assertNull;
    global.assertNotNull = TestRunner.assertNotNull;
    global.assertThrows = TestRunner.assertThrows;
    
    // Export
    global.Argon = {
        Vec: Vec,
        Option: { Some: Some, None: None },
        Result: { Ok: Ok, Err: Err },
        Shared: Shared,
        String: String,
        Map: Map,
        Set: Set,
        Test: TestRunner
    };
})(typeof globalThis !== 'undefined' ? globalThis : typeof window !== 'undefined' ? window : this);
"#
    }
}

pub fn generate_stdlib_definitions() -> HashMap<String, String> {
    let mut defs = HashMap::new();

    defs.insert(
        "Vec".to_string(),
        r#"
class Vec<T> {
    private data: T[];
    private length: usize;
    
    constructor(capacity?: usize);
    push(...items: T[]): usize;
    pop(): T | undefined;
    get(index: usize): T;
    set(index: usize, value: T): void;
    len(): usize;
    isEmpty(): boolean;
    clear(): void;
    iter(): Iterator<T>;
    map<U>(fn: (item: T, index: usize) => U): Vec<U>;
    filter(fn: (item: T, index: usize) => boolean): Vec<T>;
    reduce<U>(fn: (acc: U, item: T, index: usize) => U, initial: U): U;
}
"#
        .to_string(),
    );

    defs.insert(
        "Option".to_string(),
        r#"
type Option<T> = Some<T> | None;

class Some<T> {
    constructor(value: T);
    isSome(): boolean;
    isNone(): boolean;
    unwrap(): T;
    unwrapOr(default: T): T;
    map<U>(fn: (value: T) => U): Option<U>;
}

class None {
    isSome(): boolean;
    isNone(): boolean;
    unwrap(): never;
    unwrapOr<T>(default: T): T;
    map<U>(fn: (value: never) => U): None;
}
"#
        .to_string(),
    );

    defs.insert(
        "Result".to_string(),
        r#"
type Result<T, E> = Ok<T, E> | Err<T, E>;

class Ok<T, E> {
    constructor(value: T);
    isOk(): boolean;
    isErr(): boolean;
    unwrap(): T;
    unwrapErr(): never;
    map<U>(fn: (value: T) => U): Ok<U, E>;
    mapErr<F>(fn: (error: E) => F): Ok<T, F>;
}

class Err<T, E> {
    constructor(error: E);
    isOk(): boolean;
    isErr(): boolean;
    unwrap(): never;
    unwrapErr(): E;
    map<U>(fn: (value: T) => U): Err<U, E>;
    mapErr<F>(fn: (error: E) => F): Err<T, F>;
}
"#
        .to_string(),
    );

    defs.insert(
        "Shared".to_string(),
        r#"
class Shared<T> {
    constructor(value: T);
    clone(): Shared<T>;
    drop(): boolean;
    get(): T;
    set(value: T): void;
    
    static wrap<T>(value: T): Shared<T>;
    static unwrap<T>(shared: Shared<T>): T;
}
"#
        .to_string(),
    );

    defs.insert(
        "String".to_string(),
        r#"
class String {
    private data: string;
    
    constructor(str: string);
    len(): usize;
    isEmpty(): boolean;
    charAt(index: usize): string;
    concat(...strings: String[]): String;
    contains(substr: String): boolean;
    endsWith(suffix: String): boolean;
    startsWith(prefix: String): boolean;
    indexOf(substr: String): isize;
    slice(start: usize, end?: usize): String;
    split(separator: String): String[];
    toUpperCase(): String;
    toLowerCase(): String;
    trim(): String;
    replace(search: String, replace: String): String;
}
"#
        .to_string(),
    );

    defs.insert(
        "Map".to_string(),
        r#"
class Map<K, V> {
    constructor();
    set(key: K, value: V): Map<K, V>;
    get(key: K): V | undefined;
    has(key: K): boolean;
    delete(key: K): boolean;
    len(): usize;
    keys(): K[];
    values(): V[];
    entries(): [K, V][];
}
"#
        .to_string(),
    );

    defs.insert(
        "Set".to_string(),
        r#"
class Set<T> {
    constructor();
    add(value: T): Set<T>;
    has(value: T): boolean;
    delete(value: T): boolean;
    len(): usize;
    values(): T[];
}
"#
        .to_string(),
    );

    defs
}
