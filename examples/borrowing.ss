// Borrowing concepts - working with references
struct Point {
    x: f64;
    y: f64;
}

function getX(p: Point): f64 {
    return p.x;
}

function getY(p: Point): f64 {
    return p.y;
}

function setX(p: Point, val: f64): Point {
    return Point { x: val, y: p.y };
}

function setY(p: Point, val: f64): Point {
    return Point { x: p.x, y: val };
}

const point = Point { x: 10.0, y: 20.0 };

const x = getX(point);
const y = getY(point);
console.log(x);
console.log(y);

const moved = setX(point, 15.0);
console.log(moved.x);

// Note: SafeScript uses ownership semantics
// Shared borrows (&T) allow multiple readers
// Mutable borrows (&mut T) allow exclusive write access
// These are enforced by the borrow checker at compile time
