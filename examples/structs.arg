// Struct - stack-allocated, copied by default
struct Point {
    x: f64;
    y: f64;
}

struct Color {
    r: u8;
    g: u8;
    b: u8;
}

const origin = Point { x: 0.0, y: 0.0 };
console.log(origin.x);
console.log(origin.y);

const red = Color { r: 255, g: 0, b: 0 };
console.log(red.r);
console.log(red.g);

function getX(p: Point): f64 {
    return p.x;
}

console.log(getX(origin));
