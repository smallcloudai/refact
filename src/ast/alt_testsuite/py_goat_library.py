from typing import Optional, List, Tuple


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

    def jump_around(self):
        print(f"jump_around age={self.age} weight={self.weight}")
        self.self_review()


def animal_direct_access(v1: Goat, v2: Optional[Goat], v3: List[Goat], v4: List[Optional[Goat]]):
    print(f"animal_direct_access: age1={v1.age} age2={v2.age if v2 else 'None'} age3={[x.age for x in v3]} age4={[y.age if y else 'None' for y in v4]}")



def animal_function_calling(v1: Goat, v2: Optional[Goat], v3: List[Goat], v4: List[Optional[Goat]]):
    v1.self_review()
    if v2:
        v2.self_review()
    for x in v3:
        x.self_review()
    for y in v4:
        if y:
            y.self_review()


my_int1 = 10
my_int2: int = 11
my_int3: Optional[int] = 12
aaa1, aaa2 = 13, 14
(aaa2, aaa3) = (15, 16)
aaa5, (aaa6, aaa7) = 17, (18, 19)
