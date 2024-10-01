from typing import Optional, List


class Animal:
    def __init__(self, age):
        self.age = age

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


my_int = 5
