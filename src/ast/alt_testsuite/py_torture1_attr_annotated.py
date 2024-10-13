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

def wrapped_messy_mouse_generator(n: int) -> List[Optional[Wrapper]]:
# f ![file::Wrapper]
# p int
# U{ simple_id file::Wrapper }
    return [Wrapper(Messy()) for _ in range(n)]
    # FIX

def my_test():
# f !void
    wrapper_list1 = wrapped_messy_mouse_generator(5)
    # v [file::Wrapper]
    # U{ simple_id file::wrapped_messy_mouse_generator } U{ simple_id file::my_test::wrapper_list1 }
    wrapper_list2 = [Wrapper(Messy()) for _ in range(5)]
    # FIX v ERR/EXPR/"list_comprehension"/[Wrapper(Messy()) for _ in range(5)]
    # U{ simple_id file::my_test::wrapper_list2 }
    if 1:
        print(wrapper_list1[3].maybe().mouse)
        # U{ simple_id print } U{ simple_id file::my_test::wrapper_list1 } U{ attr file::Wrapper::maybe } U{ attr file::Messy::mouse }
        print(wrapper_list2[3].maybe().mouse)
        # U{ simple_id print } U{ simple_id file::my_test::wrapper_list2 }

my_test()
# U{ simple_id file::my_test }
