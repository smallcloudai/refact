# Picking up context, goal in this file:
# - goto parent class, two times
# - dump parent class

import frog

X,Y = 50, 50
W = 100
H = 100


# This this a comment for the Toad class, above the class
class Toad(frog.Frog):
    def __init__(self, x, y, vx, vy):
        super().__init__(x, y, vx, vy)
        self.name = "Bob"


class EuropeanCommonToad(frog.Frog):
    """
    This is a comment for EuropeanCommonToad class, inside the class
    """

    def __init__(self, x, y, vx, vy):
        super().__init__(x, y, vx, vy)
        self.name = "EU Toad"


if __name__ == "__main__":
    toad = EuropeanCommonToad(100, 100, 200, -200)
    toad.jump(W, H)
    print(toad.name, toad.x, toad.y)

