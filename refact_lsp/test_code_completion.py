from refact_lsp import refact_client
import aiohttp
import os
import termcolor


async def test_multiline(sess, no_newline_in_the_end: bool):
    example1 = "# This print hello world and does not do anything else\ndef hello_world():"
    if not no_newline_in_the_end:
        example1 += "\n"
    files = {
        "hello_world.py": example1,
    }
    completion = await refact_client.regular_code_completion(
        sess,
        files,
        "hello_world.py",
        len(example1),
        50,
        multiline=True,
    )
    print(termcolor.colored(example1, "yellow") + termcolor.colored(str(completion), "green"))
    print("checking if correct \"%s\"\n" % str(completion).replace("\n", "\\n"))
    assert completion.strip().lower().replace("!", "") in [
        "print('hello world')",
        "print(\"hello world\")",
    ]


async def test_single_line(sess):
    example2 = "# This is a simple function that adds two numbers together\n\ndef sum(a: float, b: fl):"
    cursor = len(example2) - 2   # fl|):
    files = {
        "hello_world.py": example2,
    }
    completion = await refact_client.regular_code_completion(
        sess,
        files,
        "hello_world.py",
        cursor,
        50,
        multiline=False,
    )
    print(termcolor.colored(example2[:cursor], "yellow") + termcolor.colored(str(completion), "green"))
    print("checking if correct \"%s\"\n" % str(completion).replace("\n", "\\n"))
    assert completion.strip().lower().replace("!", "") in [
        "oat):",
        "oat) -> float:",
    ]


async def test_everything():
    if "SMALLCLOUD_API_KEY" not in os.environ:
        raise ValueError("Please either set SMALLCLOUD_API_KEY environment variable or create requests session manually.")
    sess = aiohttp.ClientSession(headers={
        "Authorization": "Bearer %s" % os.environ["SMALLCLOUD_API_KEY"],
    })
    try:
        await test_multiline(sess, False)
        await test_multiline(sess, True)
        await test_single_line(sess)
    finally:
        await sess.close()

