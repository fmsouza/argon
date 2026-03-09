// SafeScript class example

class Counter {
    private value: i32;
    
    constructor(initial: i32) {
        this.value = initial;
    }
    
    increment(): void with &mut this {
        this.value = this.value + 1;
    }
    
    getValue(): i32 with &this {
        return this.value;
    }
    
    reset(): void with &mut this {
        this.value = 0;
    }
}

function main(): void {
    const counter = new Counter(10);
    
    console.log("Initial value: " + counter.getValue());
    
    counter.increment();
    counter.increment();
    counter.increment();
    
    console.log("After 3 increments: " + counter.getValue());
    
    counter.reset();
    
    console.log("After reset: " + counter.getValue());
}

main();
