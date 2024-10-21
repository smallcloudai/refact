from typing import Optional, List, Tuple, Callable
from collections import namedtuple


class Animal:
# s Animal !root::Animal
    def __init__(self, age: int):
    # f __init__() !void
    # p self root::Animal
    # p age int
        self.age = age
        # v age int
        # U{ go_up root::Animal::__init__::age } U{ attr root::Animal::age }
        self.also1_age: float = age
        # v also1_age float
        # U{ go_up root::Animal::__init__::age } U{ attr root::Animal::also1_age }
        self.also2_age = float(age)
        # v also2_age ?
        # U{ go_up root::Animal::__init__::age } U{ attr root::Animal::also2_age }
        self.also3_age = age + 5.0
        # v also3_age int
        # U{ go_up root::Animal::__init__::age } U{ attr root::Animal::also3_age }

    def self_review(self):
    # f self_review() !void
    # p self root::Animal
        print(f"self_review age={self.age}")
        # U{ go_up_fail guess ?::print } U{ attr root::Animal::age }


class Goat(Animal):
# s Goat !root::Goat
# U{ go_up root::Animal }
    def __init__(self, age: int, weight: float, *args, **kwargs):
    # ERROR py_function parameter syntax: "list_splat_pattern" in *args
    # ERROR py_function parameter syntax: "dictionary_splat_pattern" in **kwargs
    # f __init__() !void
    # p self root::Goat
    # p age int
    # p weight float
        super().__init__(age)
        # U{ go_up_fail guess ?::super } U{ attr guess ?::__init__ } U{ go_up root::Goat::__init__::age }
        self.weight = weight
        # v weight float
        # U{ go_up root::Goat::__init__::weight } U{ attr root::Goat::weight }

    def jump_around(self) -> Animal:
    # f jump_around() root::Animal
    # p self root::Goat
    # U{ go_up root::Animal }
        print(f"jump_around age={self.age} weight={self.weight}")
        # U{ go_up_fail guess ?::print } U{ attr guess ?::age } U{ attr root::Goat::weight }
        self.self_review()
        # U{ attr guess ?::self_review }
        return self

