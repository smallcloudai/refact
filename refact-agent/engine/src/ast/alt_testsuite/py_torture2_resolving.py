from sys import *
import os as ooooos
import multiprocessing
from multiprocessing import Process as NotAProcess, TimeoutError
from os. path import join as ospj, split as osps

print("ls1", ooooos.listdir("."))
print("argv", argv)


def can_you_dig_it():
    WN = WobbleNoble
    return (WN().do_the_thing(), FumbleNoble())


class WobbleNoble:
    def __init__(self):
        self.trouble = "double"

    def do_the_thing(self):
        return "wobble '%s'" % self.trouble


class FumbleNoble(object):
    def __init__(self):
        self.humble = "mumble"


def list_directory():
    import os as ooooos2
    ooooos2.system("ls")
    print("ospj", ospj("hello", "world"))


if __name__ == '__main__':
    process: multiprocessing.Process
    process = NotAProcess(target=list_directory)
    process.start()
    process.join()
    should_be_a_string, fumble = can_you_dig_it()
    print(fumble.humble)
