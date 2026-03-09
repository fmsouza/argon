// Functions in SafeScript

// Basic function with typed parameters and return type
function add(a: i32, b: i32): i32 {
    return a + b;
}

const result = add(5, 10);
console.log(result);

// Function with multiple return paths
function max(a: i32, b: i32): i32 {
    if (a > b) {
        return a;
    }
    return b;
}

console.log(max(3, 7));
console.log(max(10, 5));

// Function returning multiple values via struct
struct Pair {
    first: i32;
    second: i32;
}

function divide(a: i32, b: i32): Pair {
    if (b == 0) {
        return Pair { first: 0, second: 1 };
    }
    return Pair { first: a / b, second: 0 };
}

const div = divide(10, 2);
console.log(div.first);
console.log(div.second);

// Recursive function
function factorial(n: i32): i32 {
    if (n <= 1) {
        return 1;
    }
    return n * factorial(n - 1);
}

console.log(factorial(5));
