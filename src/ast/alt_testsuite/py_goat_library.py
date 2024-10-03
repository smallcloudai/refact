from typing import Optional, List, Tuple, Callable
from collections import namedtuple


class Animal:
    def __init__(self, age: int):
        self.age = age
        self.also1_age: float = age
        self.also2_age = float(age)
        self.also2_age = age + 5.0

    def self_review(self):
        print(f"self_review age={self.age}")

class Goat(Animal):
    def __init__(self, age: int, weight: float):
        super().__init__(age)
        self.weight = weight

    def jump_around(self) -> Animal:
        print(f"jump_around age={self.age} weight={self.weight}")
        self.self_review()
        return self


def animal_direct_access(v1: Goat, v2: Optional[Goat], v3: List[Goat], v4: List[Optional[Goat]]):
    print(f"animal_direct_access: age1={v1.age} age2={v2.age if v2 else 'None'} age3={[x.age for x in v3]} age4={[(y.age if y else 'not_a_goat') for y in v4]}")



def animal_function_calling(v1: Goat, v2: Optional[Goat], v3: List[Goat], v4: List[Optional[Goat]]):
    v1.self_review()
    if v2:
        v2.self_review()
    for x in v3:
        x.self_review()
    for y in v4:
        if y:
            y.self_review()


def goat_generator() -> Tuple[Goat, Goat]:
    return Goat(2, 2.0), Goat(3, 4.0)


my_callback: Callable[[], Tuple[Goat, Goat]]
my_callback = goat_generator
goat1, goat2 = my_callback()

my_int1 = 10
my_int2: int = 11
my_int3: Optional[int] = 12
aaa1, aaa2 = 13, 14
(aaa3, aaa4) = (15, 16)

# will not work:
aaa5, (aaa6, aaa7) = 17, (18, 19)


animal_function_calling(*goat_generator(), [Goat(4, 4.0)], [Goat(5, 5.0), None])
animal_direct_access(*goat_generator(), [Goat(4, 4.0)], [Goat(5, 5.0), None])

# Person = namedtuple('Person', ['name', 'age', 'city'])
# person1 = Person(name='Alice', age=30, city='New York')
# print(person1.name)
# my_dict = dict()
# my_dict["animal"] = Animal(5)
# my_dict["goat"] = Goat(6, 6.0)
