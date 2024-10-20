from typing import Optional, List, Tuple, Callable
from collections import namedtuple


class Animal:
# s Animal !file::Animal
    def __init__(self, age: int):
    # f __init__() !void
    # p self file::Animal
    # p age int
        self.age = age
        # v age int
        # U{ simple_id file::Animal::__init__::age } U{ attr file::Animal::age }
        self.also1_age: float = age
        # v also1_age float
        # U{ simple_id file::Animal::__init__::age } U{ attr file::Animal::also1_age }
        self.also2_age = float(age)
        # v also2_age ERR/CALL/NOT_A_THING/float
        # U{ simple_id file::Animal::__init__::age } U{ attr file::Animal::also2_age }
        self.also3_age = age + 5.0
        # v also3_age int
        # U{ simple_id file::Animal::__init__::age } U{ attr file::Animal::also3_age }

    def self_review(self):
    # f self_review() !void
    # p self file::Animal
        print(f"self_review age={self.age}")
        # U{ simple_id print }


class Goat(Animal):
# ERROR py_class syntax: "argument_list" in (Animal)
# s Goat !file::Goat
    def __init__(self, age: int, weight: float, *args, **kwargs):
    # ERROR py_function parameter syntax: "list_splat_pattern" in *args
    # ERROR py_function parameter syntax: "dictionary_splat_pattern" in **kwargs
    # f __init__() !void
    # p self file::Goat
    # p age int
    # p weight float
        super().__init__(age)
        # U{ simple_id file::Goat::__init__::age }
        self.weight = weight
        # v weight float
        # U{ simple_id file::Goat::__init__::weight } U{ attr file::Goat::weight }

    def jump_around(self) -> Animal:
    # f jump_around() file::Animal
    # p self file::Goat
    # U{ simple_id file::Animal }
        print(f"jump_around age={self.age} weight={self.weight}")
        # U{ simple_id print }
        self.self_review()
        return self

