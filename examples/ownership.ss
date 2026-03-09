// Ownership - values are moved by default for non-copyable types
struct Vec2 {
    x: f64;
    y: f64;
}

// This function takes ownership of the vector
function consume(v: Vec2): f64 {
    return v.x + v.y;
}

const v1 = Vec2 { x: 3.0, y: 4.0 };
const result = consume(v1);
console.log(result);

// After consume, v1 is moved and cannot be used
// This demonstrates ownership transfer

// Copy semantics for primitive-like types
const a: i32 = 10;
const b: i32 = a;
console.log(a);
console.log(b);
