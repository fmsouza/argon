// Pattern matching concepts
function classify(n: number): number {
    if (n < 0) {
        return 0;
    }
    if (n == 0) {
        return 1;
    }
    return 2;
}

console.log(classify(-5));
console.log(classify(0));
console.log(classify(10));
