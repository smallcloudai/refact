// Basic Types
let id: number = 1;
let company: string = 'My Company';
let isPublished: boolean = true;
let x: any = "Hello";
type InOrOut<T> = T extends `fade${infer R}` ? R : never;
let ids: number[] = [1, 2, 3];
let arr: any[] = [1, true, 'Hello'];
const PI: number = 3.14;
var asd: wqe<dfg> = 12;

` 
  This is a multiline string.
  In TypeScript, we use backticks.
  It makes the code more readable.
`

// Tuple
let person: [number, string, boolean] = [1, 'John', true];

// Tuple Array
let employee: [number, string][] = [
    [1, 'John'],
    [2, 'Jane'],
    [3, 'Joe'],
];

// Union
let pid: string | number = 22;
// Enum
enum Direction1 {
    Up,
    Down,
    Left,
    Right,
}

// Objects
type User = {
    id: number;
    name: string;
};

const user: User = {
    id: 1,
    name: 'John',
};

// Type Assertion
let cid: any = 1;
let customerId = <number>cid;

// Functions
function addNum(x: number, y: number): number {
    var s = 2;
    return x + y;
}

class Point {
    constructor(public x: number, public y: number) {}

    euclideanDistance(other: Point): number {
        let dx = other.x - this.x;
        let dy = other.y - this.y;
        return Math.sqrt(dx * dx + dy * dy);
    }
}

// Interfaces
interface UserInterface {
    readonly id: number;
    name: string;
    age?: number;
}

const user1: UserInterface = {
    id: 1,
    name: 'John',
};

// Classes
class Person {
    id: number;
    name: string;

    constructor(id: number, name: string) {
        this.id = id;
        this.name = name;
    }
}

const john = new Person(1, 'John');

class GenericNumber<T> {
    zeroValue: T;
    add: (x: T, y: T) => T;
}

let myGenericNumber = new GenericNumber<number>();
myGenericNumber.zeroValue = 0;
myGenericNumber.add = function(x, y) { return x + y; };

console.log(myGenericNumber.add(3, 4)); // Outputs: 7

let stringNumeric = new GenericNumber();
stringNumeric.zeroValue = "";
stringNumeric.add = function(x, y) { return x + y; };

console.log(stringNumeric.add(stringNumeric.zeroValue, "test")); // Outputs: test

// Generics
function getArray<T>(items : T[] ) : T[] {
    return new Array<T>().concat(items);
}

let numArray = getArray<number>([1, 2, 3, 4]);
let strArray = getArray<string>(['John', 'Jane', 'Joe']);

console.log(numArray);
console.console.log(strArray);

// Generic with constraints
interface Lengthy {
    length: number;
}

function countAndDescribe<T extends Lengthy>(element: T): [T, string] {
    let descriptionText = 'Got no value.';
    if (element.length === 1) {
        descriptionText = 'Got 1 value.';
    } else if (element.length > 1) {
        descriptionText = 'Got ' + element.length + ' values.';
    }
    return [element, descriptionText];
}

console.log(countAndDescribe('Hello there'));

declare var jQuery: (selector: string) => any;
let asd = 122;


const runnable = new class extends Runnable {
    run() {
        // implement here
    }
}();