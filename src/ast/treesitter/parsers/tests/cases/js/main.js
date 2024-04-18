// Variables
var name = "John Doe";
let age = 30;
const pi = 3.14;

// Function
function greet(name) {
    return "Hello, " + name;
}

const Color = {
    RED: 'Red',
    BLUE: 'Blue',
    GREEN: 'Green'
};

console.log(Color.RED); // Output: Red

console.log(greet(name)); // Output: Hello, John Doe

// Object
let person = {
    firstName: "John",
    lastName: "Doe",
    fullName: function() {
        return this.firstName + " " + this.lastName;
    }
}

// Declaration
class Rectangle {
    constructor(height, width) {
        this.height = height;
        this.width = width;
    }
}

// Expression; the class is anonymous but assigned to a variable
const Rectangle = class {
    #height: 1;
    #width;
    constructor(height, width) {
        this.#height = height;
        this.#width = width;
    }
};

// Expression; the class has its own name
const Rectangle = class Rectangle2 {
    height: 1;
    width: 2;
    constructor(height, width) {
        this.height = height;
        this.width = width;
    }
};


console.log(person.fullName()); // Output: John Doe
let asd = person;

// Array
let fruits = ["apple", "banana", "cherry"];
fruits.forEach(function(item, index, array) {
    console.log(item, index);
});

// Conditional
if (age > 18) {
    console.log("You are an adult.");
} else {
    console.log("You are a minor.");
}

// Loop
for (let i = 0; i < 5; i++) {
    console.log(i);
}

// Event
document.getElementById("myButton").addEventListener("click", function() {
    alert("Button clicked!");
});

// Define a class
class Person {
    constructor(firstName, lastName) {
        this.firstName = firstName;
        this.lastName = lastName;
    }

    // Method
    fullName() {
        return this.firstName + " " + this.lastName;
    }
}

// Create an instance of the class
let john = new Person("John", "Doe");

console.log(john.fullName()); // Output: John Doe

// Inheritance
class Employee extends Person {
    constructor(firstName, lastName, position) {
        super(firstName, lastName); // Call the parent constructor
        this.position = position;
    }

    // Override method
    fullName() {
        return super.fullName() + ", " + this.position;
    }
}

let jane = new Employee("Jane", "Doe", "Engineer");

console.log(jane.fullName()); // Output: Jane Doe, Engineer

// Function Declaration (or Function Statement)
function add(a, b) {
    return a + b;
}
add(2,3);
console.log(add(1, 2)); // Outputs: 3

// Function Expression
let multiply = function(a, b) {
    return a * b;
}
console.log(multiply(2, 3)); // Outputs: 6

// Arrow Function
let subtract = (a, b) => {
    return a - b;
}
console.log(subtract(5, 2)); // Outputs: 3

// Immediately Invoked Function Expression (IIFE)
(function() {
    console.log('This is an IIFE');
})(); // Outputs: This is an IIFE

// Constructor Function
function Person(name, age) {
    this.name = name;
    this.age = age;
}

let john = new Person('John', 30);
console.log(john); // Outputs: Person { name: 'John', age: 30 }

// Generator Function
function* idGenerator() {
    let id = 0;
    while (true) {
        yield id++;
    }
}

var gen = idGenerator();
console.log(gen.next().value); // Outputs: 0
console.log(gen.next().value); // Outputs: 1

import React from 'react';

class HelloWorld extends React.Component {
    render() {
        return (
            <div>
                <h1>Hello, World!</h1>
                <p>Welcome to React.</p>
            </div>
        );
    }
}

export default HelloWorld;