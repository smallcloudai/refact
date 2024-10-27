
from typing import Optional, List, Tuple, Callable
from collections import namedtuple


# s Animal !root::Animal
class Animal:
    # f __init__() !void
    # p self root::Animal
    # p age int
    def __init__(self, age: int):
        # v age int
        # U{ go_up root::Animal::__init__::age } U{ attr root::Animal::age }
        self.age = age
        # v also1_age float
        # U{ go_up root::Animal::__init__::age } U{ attr root::Animal::also1_age }
        self.also1_age: float = age
        # v also2_age ?
        # U{ go_up root::Animal::__init__::age } U{ attr root::Animal::also2_age }
        self.also2_age = float(age)
        # v also3_age int
        # U{ go_up root::Animal::__init__::age } U{ attr root::Animal::also3_age }
        self.also3_age = age + 5.0

    # f self_review() !void
    # p self root::Animal
    def self_review(self):
        # U{ go_up_fail guess ?::print } U{ attr root::Animal::age }
        print(f"self_review age={self.age}")


# s Goat !root::Goat
# U{ go_up root::Animal }
class Goat(Animal):
    # ERROR py_function parameter syntax: "list_splat_pattern" in *args
    # ERROR py_function parameter syntax: "dictionary_splat_pattern" in **kwargs
    # f __init__() !void
    # p self root::Goat
    # p age int
    # p weight float
    def __init__(self, age: int, weight: float, *args, **kwargs):
        # U{ go_up_fail guess ?::super } U{ attr guess ?::__init__ } U{ go_up root::Goat::__init__::age }
        super().__init__(age)
        # v weight float
        # U{ go_up root::Goat::__init__::weight } U{ attr root::Goat::weight }
        self.weight = weight

    # f jump_around() root::Animal
    # p self root::Goat
    # U{ go_up root::Animal }
    def jump_around(self) -> Animal:
        # U{ go_up_fail guess ?::print } U{ attr guess ?::age } U{ attr root::Goat::weight }
        print(f"jump_around age={self.age} weight={self.weight}")
        # U{ attr guess ?::self_review }
        self.self_review()
        return self
