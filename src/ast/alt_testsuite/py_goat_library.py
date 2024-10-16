from typing import Optional, List, Tuple, Callable
from collections import namedtuple


class Animal:
    def __init__(self, age: int):
        self.age = age
        self.also1_age: float = age
        self.also2_age = float(age)
        self.also3_age = age + 5.0

    def self_review(self):
        print(f"self_review age={self.age}")


class Goat(Animal):
    def __init__(self, age: int, weight: float, *args, **kwargs):
        super().__init__(age)
        self.weight = weight

    def jump_around(self) -> Animal:
        print(f"jump_around age={self.age} weight={self.weight}")
        self.self_review()
        return self

