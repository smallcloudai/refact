import os
from typing import List, Callable, Optional, Dict, Tuple
from multiprocessing import Process

# works
my_int1 = 10
my_int2: int = 11
my_int3: Optional[int] = 12
aaa1, aaa2 = 13, 14
(aaa3, aaa4) = (15, 16)

# doesn't work:
aaa5, (aaa6, aaa7) = 17, (18, 19)

class WobbleNoble:
    def __init__(self):
        self.trouble = "double"

class FumbleNoble:
    def __init__(self):
        self.humble = "mumble"


def wobble_generator(n: int) -> List[Optional[WobbleNoble]]:
    return [WobbleNoble() for i in range(n)]

wobble_generator1: Callable[[int], List[Optional[WobbleNoble]]]
wobble_generator1 = wobble_generator
wobble_generator2 = wobble_generator

def mixed_generator() -> Tuple[WobbleNoble, FumbleNoble]:
    return (WobbleNoble(), FumbleNoble())

def wobble_operator(w: Optional[WobbleNoble]) -> str:
    if w is not None:
        return w.trouble

wobble_list1 = wobble_generator1(5)
wobble_list2 = wobble_generator2(5)

if wobble_list1[3] is not None:
    print(wobble_list1[3].trouble)
if wobble_list2[3] is not None:
    print(wobble_list2[3].trouble)
print("wobble_operator", wobble_operator(wobble_list2[3]))

def mega_test() -> WobbleNoble:
    wobble, fumble = mixed_generator()
    print(wobble.trouble, fumble.humble)
    return wobble

print(mega_test().trouble)


def list_directory():
    os.system("ls")

if __name__ == '__main__':
    process = Process(target=list_directory)
    process.start()
    process.join()
