// SafeScript example with structs and functions

struct Point {
    x: f64;
    y: f64;
}

struct Circle {
    x: f64;
    y: f64;
    radius: f64;
}

function distance(p1: &Point, p2: &Point): f64 with & {
    const dx = p2.x - p1.x;
    const dy = p2.y - p1.y;
    return (dx * dx + dy * dy).sqrt();
}

function createPoint(x: f64, y: f64): Point {
    return Point { x: x, y: y };
}

function createCircle(x: f64, y: f64, radius: f64): Circle {
    return Circle { x: x, y: y, radius: radius };
}

function main(): void {
    const p1 = createPoint(0.0, 0.0);
    const p2 = createPoint(3.0, 4.0);
    const dist = distance(&p1, &p2);
    
    console.log("Distance: " + dist);
    
    const circle = createCircle(0.0, 0.0, 5.0);
    console.log("Circle radius: " + circle.radius);
}

main();
