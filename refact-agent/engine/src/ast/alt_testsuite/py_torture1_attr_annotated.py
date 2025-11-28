
from typing import List, Callable, Optional, Dict, Tuple

# s Messy !root::Messy
class Messy:
    # f __init__() !void
    # p self root::Messy
    def __init__(self):
        # v mouse str
        self.mouse = "house"
# U{ attr root::Messy::mouse }

# s Wrapper !root::Wrapper
class Wrapper:
    # f __init__() !void
    # p self root::Wrapper
    # p messy root::Messy
    def __init__(self, messy: Messy):
        # v messy root::Messy
        # U{ go_up root::Messy }
        self.messy = messy
# U{ go_up root::Wrapper::__init__::messy } U{ attr root::Wrapper::messy }

    # f maybe() !root::Messy
    # p self root::Wrapper
    def maybe(self):
        return self.messy
# U{ attr root::Wrapper::messy }

# f wrapped_messy_generator() ![root::Wrapper]
# p N int
def wrapped_messy_generator(N: int):
    # v i int
    return [Wrapper(Messy()) for i in range(N)]
# U{ go_up root::wrapped_messy_generator::N } U{ go_up root::wrapped_messy_generator::<listcomp>::i } U{ go_up root::Wrapper } U{ go_up root::Messy }

# f my_test() !void
def my_test():
    # v wrapper_list1 [root::Wrapper]
    wrapper_list1 = wrapped_messy_generator(5)
    # v _ int
    # v wrapper_list2 [root::Wrapper]
    # U{ go_up root::wrapped_messy_generator } U{ go_up root::my_test::wrapper_list1 }
    wrapper_list2 = [Wrapper(Messy()) for _ in range(5)]
    # U{ go_up root::my_test::<listcomp>::_ } U{ go_up root::Wrapper } U{ go_up root::Messy } U{ go_up root::my_test::wrapper_list2 }
    if 1:
        print(wrapper_list1[3].maybe().mouse)
        # U{ go_up_fail guess ?::print } U{ go_up root::my_test::wrapper_list1 } U{ attr root::Wrapper::maybe } U{ attr root::Messy::mouse }
        print(wrapper_list2[3].maybe().mouse)
# U{ go_up_fail guess ?::print } U{ go_up root::my_test::wrapper_list2 } U{ attr root::Wrapper::maybe } U{ attr root::Messy::mouse }

my_test()