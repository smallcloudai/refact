#include <stdio.h>
#include <memory>

class Animal {
public:
    int age;

    Animal(int age_)
    {
        age = age_;
    }
};

class HasMass {
public:
    float mass;

    HasMass(float mass):
        mass(mass)
    {
    }
};

class CompiledFrog: public Animal, public HasMass {
public:
    CompiledFrog(int age, float mass):
        Animal(age),
        HasMass(mass)
    {
    }

    void say_hi() const
    {
        printf("I am a frog! age=%d mass=%0.2f\n", age, mass);
    }
};

static CompiledFrog global_frog(8, 888.0);

void some_fun(CompiledFrog* f1, CompiledFrog& f2, const CompiledFrog& f3, const std::shared_ptr<CompiledFrog>& f4)
{
    CompiledFrog f_local_frog(7, 666.0);
    f1->say_hi();
    f2.say_hi();
    f3.say_hi();
    f4->say_hi();
    f_local_frog.say_hi();
    global_frog.say_hi();
}

void some_variable_usage(CompiledFrog* v1, CompiledFrog& v2, const CompiledFrog& v3, const std::shared_ptr<CompiledFrog>& v4)
{
    CompiledFrog v_local_frog(9, 999.0);
    v1->mass;
    v2.mass;
    v3.mass;
    v4->mass;
    v_local_frog.mass;
    global_frog.mass;
}

int main()
{
    CompiledFrog teh_frog(5, 13.37);
    std::shared_ptr<CompiledFrog> shared_frog = std::make_shared<CompiledFrog>(6, 42.0);
    some_fun(&teh_frog, teh_frog, teh_frog, shared_frog);
    some_variable_usage(&teh_frog, teh_frog, teh_frog, shared_frog);
}
