import os
import datetime
import random
import logging


_random_chars = "0123456789" + "ABCDEFGHIJKLNMPQRSTUVWXYZ" + "ABCDEFGHIJKLNMPQRSTUVWXYZ".lower()


def random_guid(n=12):
    guid = "".join([_random_chars[random.randint(0, len(_random_chars)-1)] for _ in range(n)])
    return guid


log = logging.getLogger("WEBUI").info


def clamp(lower, upper, x):
    return max(lower, min(upper, x))
