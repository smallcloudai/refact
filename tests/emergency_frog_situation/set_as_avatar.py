# Picking up context, goal in this file:
# - goto parent class, two times
# - dump parent class

import frog


# This this a comment for the Toad class, above the class
class Toad(frog.Frog):
    def __init__(self, x, y, vx, vy):
        super().__init__(x, y, vx, vy)
        self.known_as = "Bob"
        self.croak()

    def hello_world(self):
        self.croak()


class EuropeanCommonToad(frog.Frog):
    """
    This is a comment for EuropeanCommonToad class, inside the class
    """

    def __init__(self, x, y, vx, vy):
        super().__init__(x, y, vx, vy)
        self.known_as = "EU Toad"


def some_fun(f1: Toad, f2: EuropeanCommonToad, f3: frog.Frog, f4):
    f1.croak()
    f2.croak()
    f3.croak()
    f4.croak()

def use_some_variables(f1: Toad, f2: EuropeanCommonToad, f3: frog.Frog, f4):
    print(f1.known_as)
    print(f2.known_as)
    print(f3.known_as)  # there isn't one in Frog!
    print(f3.x)         # but it has x
    print(f4.y)         # no type, can't resolve

def a_bigger_test():
    f1 = Toad(110, 110, 0.2, 0.4)
    f2 = EuropeanCommonToad(120, 120, 0.3, 0.4)
    f3 = frog.Frog(130, 130, 0.4, 0.6)
    f4 = f3
    some_fun(f1, f2, f3, f4)
    use_some_variables(f1, f2, f3, f4)


if __name__ == "__main__":
    toad = EuropeanCommonToad(100, 100, 200, -200)
    toad.jump(3, 4)
    print(toad.known_as, toad.x, toad.y)
