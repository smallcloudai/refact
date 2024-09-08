#include "cpp_goat_library.h"

class CosmicJustice {
public:
    float balance;

    CosmicJustice()
    {
        balance = 0;
    }
};

class CosmicGoat: public Goat, public CosmicJustice {
public:
    CosmicGoat(int age, int weight, float balance_):
        Goat(age, weight),
        CosmicJustice()
    {
        balance = balance_;
    }

    void say_hi() const
    {
        printf("I am a CosmicGoat, age=%d weight=%d balance=%0.2f\n", age, weight, balance);
    }
};


CosmicGoat* goat_generator1()
{
    CosmicGoat* a1 = new CosmicGoat(10, 20, 30.5);
    return a1;
}

CosmicGoat goat_generator2()
{
    return CosmicGoat(11, 21, 31.5);
}

std::shared_ptr<CosmicGoat> goat_generator3()
{
    return std::make_shared<CosmicGoat>(12, 22, 32.5);
}

static CosmicGoat global_goat(13, 23, 33.5);

void all_goats_say_hi(CosmicGoat* f1, CosmicGoat& f2, const CosmicGoat& f3, const std::shared_ptr<CosmicGoat>& f4)
{
    CosmicGoat f_local_frog(14, 24, 34.5);
    f1->say_hi();
    f2.say_hi();
    f3.say_hi();
    f4->say_hi();
    f_local_frog.say_hi();
    global_goat.say_hi();
}

void all_goats_review(CosmicGoat* f1, CosmicGoat& f2, const CosmicGoat& f3, const std::shared_ptr<CosmicGoat>& f4)
{
    CosmicGoat f_local_goat(15, 25, 35.5);
    f1->self_review();
    f2.self_review();
    f3.self_review();
    f4->self_review();
    f_local_goat.self_review();
    global_goat.self_review();
}

int goat_direct_access(CosmicGoat* v1, CosmicGoat& v2, const CosmicGoat& v3, const std::shared_ptr<CosmicGoat>& v4)
{
    CosmicGoat v_local_goat(16, 26, 36.5);
    return v1->weight + v2.weight + v3.weight + v4->weight + global_goat.weight + v_local_goat.weight;
}

int goat_balance_sum(CosmicGoat* v1, CosmicGoat& v2, const CosmicGoat& v3, const std::shared_ptr<CosmicGoat>& v4)
{
    CosmicGoat v_local_goat(16, 26, 36.5);
    return v1->balance + v2.balance + v3.balance + v4->balance + global_goat.balance + v_local_goat.balance;
}

int main()
{
    CosmicGoat* goat1 = goat_generator1();
    CosmicGoat goat2 = goat_generator2();
    std::shared_ptr<CosmicGoat> goat3 = goat_generator3();

    all_goats_say_hi(goat1, goat2, goat2, goat3);
    all_goats_review(goat1, goat2, goat2, goat3);
    printf("goat_direct_access %d\n", goat_direct_access(goat1, goat2, goat2, goat3));
    printf("goat_balance_sum %d\n", goat_balance_sum(goat1, goat2, goat2, goat3));

    delete goat1;
}
