from typing import List, Callable, Optional, Dict, Tuple

class Messy:
# s Messy !file::Messy
    def __init__(self):
    # f __init__() !void
    # p self file::Messy
        self.mouse = "house"
        # v mouse str
        # U{ attr file::Messy::mouse }

class Wrapper:
# s Wrapper !file::Wrapper
    def __init__(self, messy: Messy):
    # f __init__() !void
    # p self file::Wrapper
    # p messy file::Messy
    # U{ simple_id file::Messy }
        self.messy = messy
        # v messy file::Messy
        # U{ simple_id file::Wrapper::__init__::messy } U{ attr file::Wrapper::messy }

    def maybe(self):
    # f maybe() !file::Messy
    # p self file::Wrapper
        return self.messy
        # U{ attr file::Wrapper::messy }

def wrapped_messy_generator(N: int):
# f wrapped_messy_generator() ![file::Wrapper]
# p N int
    return [Wrapper(Messy()) for i in range(N)]
    # v i ERR/FUNC_NOT_FOUND/range
    # U{ simple_id file::wrapped_messy_generator::N } U{ simple_id file::wrapped_messy_generator::<listcomp>::i } U{ simple_id file::Wrapper } U{ simple_id file::Messy }

def my_test():
# f my_test() !void
    wrapper_list1 = wrapped_messy_generator(5)
    # v wrapper_list1 [file::Wrapper]
    # U{ simple_id file::wrapped_messy_generator } U{ simple_id file::my_test::wrapper_list1 }
    wrapper_list2 = [Wrapper(Messy()) for _ in range(5)]
    # v _ ERR/FUNC_NOT_FOUND/range
    # v wrapper_list2 [file::Wrapper]
    # U{ simple_id file::my_test::<listcomp>::_ } U{ simple_id file::Wrapper } U{ simple_id file::Messy } U{ simple_id file::my_test::wrapper_list2 }
    if 1:
        print(wrapper_list1[3].maybe().mouse)
        # U{ simple_id print } U{ simple_id file::my_test::wrapper_list1 } U{ attr file::Wrapper::maybe } U{ attr file::Messy::mouse }
        print(wrapper_list2[3].maybe().mouse)
        # U{ simple_id print } U{ simple_id file::my_test::wrapper_list2 } U{ attr file::Wrapper::maybe } U{ attr file::Messy::mouse }

my_test()
# U{ simple_id file::my_test }
