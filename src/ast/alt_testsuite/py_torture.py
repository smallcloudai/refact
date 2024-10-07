import os, sys
from typing import List, Callable, Optional, Dict, Tuple
import os as ooooos
import multiprocessing
from multiprocessing import Process as NotAProcess, TimeoutError
from os. path import join as ospj, split as osps

print("ls1", ooooos.listdir("."))

# works
my_int1 = 10
my_int2: int = 11
my_int2 = 22
my_int3: Optional[int] = 12
my_int4 = sum([1337])
aaa1, aaa2 = 13, 14
(aaa3, aaa4) = (15, 16)
aaa5: Tuple[int, float] = (20, 21)

# doesn't work:
aaa6, (aaa7, aaa8) = 16, (17, 18)

class WobbleNoble:
    def __init__(self):
        self.trouble = "double"

class FumbleNoble(object):
    def __init__(self):
        self.humble = "mumble"


def wobble_generator(n: int) -> List[Optional[WobbleNoble]]:
    return [WobbleNoble() for _ in range(n)]

wobble_generator1: Callable[[int], List[Optional[WobbleNoble]]]
wobble_generator1 = wobble_generator
wobble_generator2 = wobble_generator

def mixed_generator() -> Tuple[WobbleNoble, FumbleNoble]:
    return (WobbleNoble(), FumbleNoble())

def wobble_operator(w: Optional[WobbleNoble]) -> str:
    if w is not None:
        return w.trouble
    return "woof"

wobble_list1 = wobble_generator1(5)
wobble_list2 = wobble_generator2(5)

if wobble_list1[3] is not None:
    print(wobble_list1[3].trouble)
if wobble_list2[3] is not None:
    print(wobble_list2[3].trouble)
print("wobble_operator", wobble_operator(wobble_list2[3]))
print("wobble_operator", wobble_operator(None))

def mega_test() -> WobbleNoble:
    wobble, fumble = mixed_generator()
    print(wobble.trouble, fumble.humble)
    return wobble

print(mega_test().trouble)


def list_directory():
    import os as ooooos2
    ooooos2.system("ls")


if __name__ == '__main__':
    process: multiprocessing.Process
    process = NotAProcess(target=list_directory)
    process.start()
    process.join()
