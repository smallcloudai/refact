import numpy as np

DT = 0.01


class Toad:
    def __init__(self, x, y, vx, vy):
        self.x = x
        self.y = y
        self.vx = vx
        self.vy = vy

    def bounce_off_banks(self, pond_width, pond_height):
        pass


    def jump(self, pond_width, pond_height):
        self.x += self.vx * DT
        self.y += self.vy * DT
        self.bounce_off_banks(pond_width, pond_height)
        self.x = np.clip(self.x, 0, pond_width)
        self.y = np.clip(self.y, 0, pond_height)

    def croak(self, n_times):
        for n in range(n_times):
            print("croak")


class AlternativeFrog:
    def alternative_jump(self):
        pass


def standalone_jumping_function():
    print("I'm a frog! Jump! Jump! Really!")
