/**
 * This class represents a car object.
 */
class Car {
    /**
     * Constructor for creating a new Car object.
     * @param {string} make - The make of the car.
     * @param {string} model - The model of the car.
     */
    constructor(make, model) {
        this.make = make;
        this.model = model;
        this.speed = 0;
    }

    /**
     * Method to accelerate the car.
     * @param {number} increment - The amount by which to increase the speed.
     */
    accelerate(increment) {
        this.speed += increment;
    }

    /**
     * Method to decelerate the car.
     * @param {number} decrement - The amount by which to decrease the speed.
     */
    decelerate(decrement) {
        this.speed -= decrement;
    }

    /**
     * Method to get the current speed of the car.
     * @returns {number} The current speed of the car.
     */
    getSpeed() {
        return this.speed;
    }
}