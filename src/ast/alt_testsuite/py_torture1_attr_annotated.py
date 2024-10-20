from typing import List, Callable, Optional, Dict, Tuple

class Messy:
# s Messy !root::Messy
    def __init__(self):
    # f __init__() !void
    # p self root::Messy
        self.mouse = "house"
        # v mouse str
        # U{ attr root::Messy::mouse }

class Wrapper:
# s Wrapper !root::Wrapper
    def __init__(self, messy: Messy):
    # f __init__() !void
    # p self root::Wrapper
    # p messy root::Messy
    # U{ simple_id root::Messy }
        self.messy = messy
        # v messy root::Messy
        # U{ simple_id root::Wrapper::__init__::messy } U{ attr root::Wrapper::messy }

    def maybe(self):
    # f maybe() !root::Messy
    # p self root::Wrapper
        return self.messy
        # U{ attr root::Wrapper::messy }

def wrapped_messy_generator(N: int):
# f wrapped_messy_generator() ![root::Wrapper]
# p N int
    return [Wrapper(Messy()) for i in range(N)]
    # v i ERR/FUNC_NOT_FOUND/range
    # U{ simple_id root::wrapped_messy_generator::N } U{ simple_id root::wrapped_messy_generator::<listcomp>::i } U{ simple_id root::Wrapper } U{ simple_id root::Messy }

def my_test():
# f my_test() !void
    wrapper_list1 = wrapped_messy_generator(5)
    # v wrapper_list1 [root::Wrapper]
    # U{ simple_id root::wrapped_messy_generator } U{ simple_id root::my_test::wrapper_list1 }
    wrapper_list2 = [Wrapper(Messy()) for _ in range(5)]
    # v _ ERR/FUNC_NOT_FOUND/range
    # v wrapper_list2 [root::Wrapper]
    # U{ simple_id root::my_test::<listcomp>::_ } U{ simple_id root::Wrapper } U{ simple_id root::Messy } U{ simple_id root::my_test::wrapper_list2 }
    if 1:
        print(wrapper_list1[3].maybe().mouse)
        # U{ simple_id print } U{ simple_id root::my_test::wrapper_list1 } U{ attr root::Wrapper::maybe } U{ attr root::Messy::mouse }
        print(wrapper_list2[3].maybe().mouse)
        # U{ simple_id print } U{ simple_id root::my_test::wrapper_list2 } U{ attr root::Wrapper::maybe } U{ attr root::Messy::mouse }

my_test()
# U{ simple_id root::my_test }
