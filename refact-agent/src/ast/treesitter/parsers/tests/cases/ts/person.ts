/**
 * This is a class representing a Person.
 */
class Person {
    private name: string;
    private age: number;

    /**
     * This is the constructor method for the Person class.
     * @param name The name of the person.
     * @param age The age of the person.
     */
    constructor(name: string, age: number) {
        this.name = name;
        this.age = age;
    }

    /**
     * This method returns the name of the person.
     * @returns The name of the person.
     */
    getName(): string {
        return this.name;
    }

    /**
     * This method returns the age of the person.
     * @returns The age of the person.
     */
    getAge(): number {
        return this.age;
    }

    /**
     * This method sets the name of the person.
     * @param name The new name of the person.
     */
    setName(name: string): void {
        this.name = name;
    }

    /**
     * This method sets the age of the person.
     * @param age The new age of the person.
     */
    setAge(age: number): void {
        this.age = age;
    }
}