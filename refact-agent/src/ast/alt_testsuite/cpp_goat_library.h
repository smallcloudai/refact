#include <stdio.h>
#include <memory>

class Animal {
public:
    int age;

    Animal(int age_)
    {
        age = age_;
    }

    void self_review() const
    {
        printf("self_review age=%d\n", age);
    }
};

class Goat: public Animal {
public:
    int weight;

    Goat(int age_, int weight_):
        Animal(age_),
        weight(weight_)
    {
    }

    void jump_around()
    {
        printf("jump_around age=%d weight=%d\n", age, weight);
        self_review();
    }
};

inline void animal_direct_access(Animal* v1, Animal& v2, const Animal& v3, const std::shared_ptr<Animal>& v4)
{
    printf("animal_direct_access: age1=%d age2=%d age3=%d age4=%d\n",
        v1->age,
        v2.age,
        v3.age,
        v4->age
    );
}

inline void animal_function_calling(Animal* f1, Animal& f2, const Animal& f3, const std::shared_ptr<Animal>& f4)
{
    f1->self_review();
    f2.self_review();
    f3.self_review();
    f4->self_review();
}
