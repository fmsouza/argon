// Option type - represents optional values using structs
struct Some {
    value: i32;
}

struct None {
}

// Simulating Option with pattern matching
function findUser(id: i32): Some {
    if (id > 0) {
        return Some { value: id };
    }
    return None { };
}

const found = findUser(42);
const missing = findUser(-1);

console.log(found.value);

// Result type - for error handling  
struct Ok {
    value: f64;
}

struct Err {
    error: string;
}

function divide(a: f64, b: f64): Ok {
    if (b == 0.0) {
        return Err { error: "Division by zero" };
    }
    return Ok { value: a / b };
}

const success = divide(10.0, 2.0);
const failure = divide(10.0, 0.0);

console.log(success.value);
