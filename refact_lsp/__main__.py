from refact_lsp import refact_lsp_server
from refact_lsp import test_code_completion
import argparse
import asyncio
import logging


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--test', action='store_true', help='Run tests')
    parser.add_argument('--tcp-server', type=int, help='Listen on 127.0.0.1 on the specified port, useful for debugging')
    args = parser.parse_args()
    # logging.basicConfig(level=logging.DEBUG)

    loop = asyncio.new_event_loop()
    if args.test:
        loop.run_until_complete(test_code_completion.test_everything())
    elif args.tcp_server:
        print("listening on 127.0.0.1 on port %d" % args.tcp_server)
        refact_lsp_server.server.start_tcp("127.0.0.1", args.tcp_server)
    else:
        parser.print_help()


# TODO:
# * allow empty model
# * allow no temperature
# * /contrast should return mime type json

