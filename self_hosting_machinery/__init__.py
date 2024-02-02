import sys
import logging

from self_hosting_machinery.scripts import env


NOTICE = logging.INFO + 1


def init_logger(name: str):
    logging.addLevelName(NOTICE, "NOTICE")
    logging.basicConfig(
        level=NOTICE,
        format=f'%(levelname)s %(asctime)s {name} %(message)s',
        datefmt='%Y%m%d %H:%M:%S',
        handlers=[logging.StreamHandler(stream=sys.stderr)])