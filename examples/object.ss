// Object-like structures using structs

struct PersonName {
    name: string;
}

struct PersonAge {
    age: i32;
}

const personName = PersonName { name: "Alice" };
const personAge = PersonAge { age: 30 };

console.log(personName.name);
console.log(personAge.age);
