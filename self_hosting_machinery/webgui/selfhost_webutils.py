import logging

from functools import partial

from self_hosting_machinery import NOTICE


logger = logging.getLogger("WEBUI")
log = partial(logger.log, NOTICE)
