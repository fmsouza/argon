// Class syntax demonstration
// Note: Methods on class instances need proper 'new' keyword which is a codegen issue

class Greeter {
    message: string;
    
    constructor(msg: string) {
        this.message = msg;
    }
}

const greeter = Greeter { message: "Hello" };
console.log(greeter.message);
