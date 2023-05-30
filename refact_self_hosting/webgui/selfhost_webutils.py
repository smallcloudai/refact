import os
import datetime
import random


_random_chars = "0123456789" + "ABCDEFGHIJKLNMPQRSTUVWXYZ" + "ABCDEFGHIJKLNMPQRSTUVWXYZ".lower()


def random_guid(n=12):
    guid = "".join([_random_chars[random.randint(0, len(_random_chars)-1)] for _ in range(n)])
    return guid


def log(*args):
    tmp = " ".join(str(x) for x in args)
    ymd = datetime.datetime.now().strftime("%Y%m%d")
    hms = datetime.datetime.now().strftime("%H%M%S.%f")
    print("%s %s" % (hms, tmp))
    if os.path.exists("/home/user/"):
        with open("/home/user/uvicorn-pymsg-%s.log" % ymd, "a") as f:
            f.write("%s %s\n" % (hms, tmp))

