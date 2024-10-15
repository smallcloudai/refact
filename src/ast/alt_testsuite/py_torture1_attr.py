from typing import List, Callable, Optional, Dict, Tuple

class Messy:
    def __init__(self):
        self.mouse = "house"

class Wrapper:
    def __init__(self, messy: Messy):
        self.messy = messy

    def maybe(self):
        return self.messy

def wrapped_messy_generator(N: int):
    return [Wrapper(Messy()) for i in range(N)]

def my_test():
    wrapper_list1 = wrapped_messy_generator(5)
    wrapper_list2 = [Wrapper(Messy()) for _ in range(5)]
    if 1:
        print(wrapper_list1[3].maybe().mouse)
        print(wrapper_list2[3].maybe().mouse)

my_test()
