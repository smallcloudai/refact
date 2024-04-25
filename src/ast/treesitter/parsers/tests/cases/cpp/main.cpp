#include <iostream>  // system header file
#include "myheader.h"  // your own header file
#include <vector>
#include <algorithm>

void func(int a) { }
void (*ptr)(zxc, sfd::sdfg) = &func;
(*ptr)(5); // call the function with value 5

auto object = new struct {
    int field1;
    float field2;
}();

object->field1 = 10;
object->field2 = 20.5f;

auto lambda = [](int x, int y) { return x + y; };
int sum = lambda(5, 10);  // sum will be 15

// Namespace
using namespace std;

// Class definition
class Animal {
public:
    // Constructor
    Animal(string n) : name(n) {}

    // Virtual function
    virtual void makeSound() const {
        cout << name << " makes a sound." << endl;
    }

    // Accessor
    string getName() const {
        return name;
    }

private:
    string name;
};

// Inheritance
class Dog : public Animal {
public:
    Dog(string n) : Animal(n) {}

    // Polymorphism
    void makeSound() const override {
        cout << getName() << " barks." << endl;
    }
};

int main() {
    // Dynamic memory
    Animal* pet1 = new Animal("Pet");
    Dog* pet2 = new Dog("Dog");

    // STL container
    vector<Animal*> pets = {pet1, pet2};

    // Loop
    for (Animal* pet : pets) {
        // Polymorphism
        pet->makeSound();
    }

    // Memory cleanup
    delete pet1;
    delete pet2;

    return 0;
}