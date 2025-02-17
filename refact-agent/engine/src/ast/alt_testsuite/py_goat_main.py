from py_goat_library import Animal
from typing import Optional, List, Tuple
import py_goat_library


class CosmicJustice:
    def __init__(self):
        self.balance = 0.0


class CosmicGoat(py_goat_library.Goat, CosmicJustice):
    def __init__(self, age, weight, balance_):
        py_goat_library.Goat.__init__(self, age, weight)
        CosmicJustice.__init__(self)
        self.balance = balance_

    def say_hi(self):
        print(f"I am a CosmicGoat, age={self.age} weight={self.weight} balance={self.balance:.2f}")


def goat_generator1():
    return CosmicGoat(10, 20, 30.5)

def goat_generator2():
    return CosmicGoat(11, 21, 31.5)


def animal_direct_access(v1: CosmicGoat, v2: Optional[Animal], v3: List[Animal], v4: List[Optional[Animal]]):
    print(f"animal_direct_access: age1={v1.age} age2={v2.age if v2 else 'None'} age3={[x.age for x in v3]} age4={[(y.age if y else 'not_a_goat') for y in v4]}")


def animal_function_calling(v1: CosmicGoat, v2: Optional[Animal], v3: List[Animal], v4: List[Optional[Animal]]):
    v1.self_review()
    if v2:
        v2.self_review()
    for x in v3:
        x.self_review()
    for y in v4:
        if y:
            y.self_review()


def goat_generator() -> Tuple[CosmicGoat, CosmicGoat]:
    return CosmicGoat(2, 2.0, 13.37), CosmicGoat(3, 4.0, 13.37)


if __name__ == '__main__':
    animal_function_calling(*goat_generator(), [CosmicGoat(4, 4.0, 13.37)], [CosmicGoat(5, 5.0, 13.37), None])
    goat_generator_copy = goat_generator
    animal_direct_access(*goat_generator_copy(), [CosmicGoat(4, 4.0, 13.37)], [CosmicGoat(5, 5.0, 13.37), None])
