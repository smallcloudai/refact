from py_goat_library import Animal
from typing import Optional, List, Tuple
import py_goat_library


class CosmicJustice:
# s CosmicJustice !root::CosmicJustice
    def __init__(self):
    # f __init__() !void
    # p self root::CosmicJustice
        self.balance = 0.0
        # v balance float
        # U{ attr root::CosmicJustice::balance }


class CosmicGoat(py_goat_library.Goat, CosmicJustice):
# s CosmicGoat !root::CosmicGoat
# U{ alias ?::py_goat_library } U{ attr guess ?::Goat } U{ go_up root::CosmicJustice }
    def __init__(self, age, weight, balance_):
    # f __init__() !void
    # p self root::CosmicGoat
    # p age ?
    # p weight ?
    # p balance_ ?
        py_goat_library.Goat.__init__(self, age, weight)
        # U{ alias ?::py_goat_library } U{ attr guess ?::Goat } U{ attr guess ?::__init__ } U{ go_up root::CosmicGoat::__init__::age } U{ go_up root::CosmicGoat::__init__::weight }
        CosmicJustice.__init__(self)
        # U{ go_up root::CosmicJustice } U{ attr guess ?::__init__ }
        self.balance = balance_
        # v balance ?
        # U{ go_up root::CosmicGoat::__init__::balance_ } U{ attr root::CosmicGoat::balance }

    def say_hi(self):
    # f say_hi() !void
    # p self root::CosmicGoat
        print(f"I am a CosmicGoat, age={self.age} weight={self.weight} balance={self.balance:.2f}")
        # U{ go_up_fail guess ?::print } U{ attr guess ?::age } U{ attr guess ?::weight } U{ attr root::CosmicGoat::balance }


def goat_generator1():
# f goat_generator1() !root::CosmicGoat
    return CosmicGoat(10, 20, 30.5)
    # U{ go_up root::CosmicGoat }

def goat_generator2():
# f goat_generator2() !root::CosmicGoat
    return CosmicGoat(11, 21, 31.5)
    # U{ go_up root::CosmicGoat }


def animal_direct_access(v1: CosmicGoat, v2: Optional[Animal], v3: List[Animal], v4: List[Optional[Animal]]):
# f animal_direct_access() !void
# p v1 root::CosmicGoat
# p v2 ?::py_goat_library::Animal
# p v3 [?::py_goat_library::Animal]
# p v4 [?::py_goat_library::Animal]
# U{ go_up root::CosmicGoat } U{ alias ?::py_goat_library::Animal } U{ alias ?::py_goat_library::Animal } U{ alias ?::py_goat_library::Animal }
    print(f"animal_direct_access: age1={v1.age} age2={v2.age if v2 else 'None'} age3={[x.age for x in v3]} age4={[(y.age if y else 'not_a_goat') for y in v4]}")
    # ERROR py_type_of_expr syntax: "conditional_expression" in v2.age if v2 else 'None'
    # ERROR py_type_of_expr syntax: "parenthesized_expression" in (y.age if y else 'not_a_goat')
    # v x ?::py_goat_library::Animal
    # v y ?::py_goat_library::Animal
    # U{ go_up_fail guess ?::print } U{ go_up root::animal_direct_access::v1 } U{ attr guess ?::age } U{ go_up root::animal_direct_access::v3 } U{ go_up root::animal_direct_access::<listcomp>::x } U{ go_up root::animal_direct_access::<listcomp>::x } U{ attr guess ?::age } U{ go_up root::animal_direct_access::v4 } U{ go_up root::animal_direct_access::<listcomp>::y }


def animal_function_calling(v1: CosmicGoat, v2: Optional[Animal], v3: List[Animal], v4: List[Optional[Animal]]):
# f animal_function_calling() !void
# p v1 root::CosmicGoat
# p v2 ?::py_goat_library::Animal
# p v3 [?::py_goat_library::Animal]
# p v4 [?::py_goat_library::Animal]
# U{ go_up root::CosmicGoat } U{ alias ?::py_goat_library::Animal } U{ alias ?::py_goat_library::Animal } U{ alias ?::py_goat_library::Animal }
    v1.self_review()
    # U{ go_up root::animal_function_calling::v1 } U{ attr guess ?::self_review }
    if v2:
    # ERROR py_body no body: "identifier" in v2
        v2.self_review()
        # U{ go_up root::animal_function_calling::v2 } U{ attr guess ?::self_review }
    for x in v3:
    # v x ?::py_goat_library::Animal
    # U{ go_up root::animal_function_calling::v3 } U{ go_up root::animal_function_calling::x }
        x.self_review()
    for y in v4:
    # v y ?::py_goat_library::Animal
    # U{ go_up root::animal_function_calling::v4 } U{ go_up root::animal_function_calling::y }
        if y:
            y.self_review()


def goat_generator() -> Tuple[CosmicGoat, CosmicGoat]:
# f goat_generator() (root::CosmicGoat,root::CosmicGoat)
# U{ go_up root::CosmicGoat } U{ go_up root::CosmicGoat }
    return CosmicGoat(2, 2.0, 13.37), CosmicGoat(3, 4.0, 13.37)
    # U{ go_up root::CosmicGoat } U{ go_up root::CosmicGoat }


if __name__ == '__main__':
    animal_function_calling(*goat_generator(), [CosmicGoat(4, 4.0, 13.37)], [CosmicGoat(5, 5.0, 13.37), None])
    # ERROR py_type_of_expr syntax: "list_splat" in *goat_generator()
    # ERROR py_type_of_expr syntax: "list" in [CosmicGoat(4, 4.0, 13.37)]
    # ERROR py_type_of_expr syntax: "list" in [CosmicGoat(5, 5.0, 13.37), None]
    # U{ go_up root::animal_function_calling }
    goat_generator_copy = goat_generator
    # v goat_generator_copy (root::CosmicGoat,root::CosmicGoat)
    # U{ go_up root::goat_generator } U{ go_up root::goat_generator_copy }
    animal_direct_access(*goat_generator_copy(), [CosmicGoat(4, 4.0, 13.37)], [CosmicGoat(5, 5.0, 13.37), None])
    # ERROR py_type_of_expr syntax: "list_splat" in *goat_generator_copy()
    # ERROR py_type_of_expr syntax: "list" in [CosmicGoat(4, 4.0, 13.37)]
    # ERROR py_type_of_expr syntax: "list" in [CosmicGoat(5, 5.0, 13.37), None]
    # U{ go_up root::animal_direct_access }
