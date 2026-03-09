// Numeric types in SafeScript
// All numeric types map to JavaScript numbers at runtime
// but are distinct types for type checking and memory layout

const i8val: i8 = 127;
const i16val: i16 = 32767;
const i32val: i32 = 2147483647;
const u8val: u8 = 255;
const u16val: u16 = 65535;
const u32val: u32 = 4294967295;
const f32val: f32 = 3.14159;
const f64val: f64 = 3.14159265358979;

console.log(i8val);
console.log(i16val);
console.log(i32val);
console.log(u8val);
console.log(u16val);
console.log(u32val);
console.log(f32val);
console.log(f64val);

// Arithmetic with numeric types
const sum: i64 = i32val + u32val;
const product: i32 = i16val * u8val;
const quotient: f64 = f64val / 2.0;

console.log(sum);
console.log(product);
console.log(quotient);
