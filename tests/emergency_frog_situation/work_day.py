# Picking up context, goal in this file:
# - without any other information, find method usage in another file by text similarity

import numpy as np
import frog

X,Y = 50, 50
W = 100
H = 100

def bring_your_own_frog_to_work_day(f):
    f.jump(W, H)
