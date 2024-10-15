from typing import List, Callable, Optional, Dict, Tuple

class Messy:
# s !file::Messy
    def __init__(self):
    # f !void
    # p file::Messy
        self.mouse = "house"
        # v str
        # U{ attr file::Messy::mouse }

class Wrapper:
# s !file::Wrapper
    def __init__(self, messy: Messy):
    # f !void
    # p file::Wrapper
    # p file::Messy
    # U{ simple_id file::Messy }
        self.messy = messy
        # v file::Messy
        # U{ simple_id file::Wrapper::__init__::messy } U{ attr file::Wrapper::messy }

    def maybe(self):
    # f !file::Messy
    # p file::Wrapper
        return self.messy
        # U{ attr file::Wrapper::messy }

def wrapped_messy_mouse_generator(N: int):
# f ![file::Wrapper]
# p int
    return [Wrapper(Messy()) for i in range(N)]
    # v ERR/FUNC_NOT_FOUND/range
    # U{ simple_id file::wrapped_messy_mouse_generator::N } U{ simple_id file::wrapped_messy_mouse_generator::<listcomp>::i } U{ simple_id file::Wrapper } U{ simple_id file::Messy }

def my_test():
# f !void
    wrapper_list1 = wrapped_messy_mouse_generator(5)
    # v [file::Wrapper]
    # U{ simple_id file::wrapped_messy_mouse_generator } U{ simple_id file::my_test::wrapper_list1 }
    wrapper_list2 = [Wrapper(Messy()) for _ in range(5)]
    # v ERR/FUNC_NOT_FOUND/range
    # v [file::Wrapper]
    # U{ simple_id file::my_test::<listcomp>::_ } U{ simple_id file::Wrapper } U{ simple_id file::Messy } U{ simple_id file::my_test::wrapper_list2 }
    if 1:
        print(wrapper_list1[3].maybe().mouse)
        # U{ simple_id print } U{ simple_id file::my_test::wrapper_list1 } U{ attr file::Wrapper::maybe } U{ attr file::Messy::mouse }
        print(wrapper_list2[3].maybe().mouse)
        # U{ simple_id print } U{ simple_id file::my_test::wrapper_list2 } U{ attr file::Wrapper::maybe } U{ attr file::Messy::mouse }

my_test()
# U{ simple_id file::my_test }
