import termcolor
import refact_encoding
from refact_encoding.encoding import RefactEncoding


def test_long_text(enc):
    long_text = open(refact_encoding.encoding.__file__).read()
    toks = enc.encode(long_text)
    print("\nlong text tokens", len(toks))
    # assert enc.LFLF not in toks
    # for i, tok in enumerate(toks):
    #     print("%03i %i \"%s\"" % (i, tok, enc.decode([tok]).replace("\n", "\\n")))
    assert long_text == enc.decode(toks)


def test_position_tokens(enc: RefactEncoding):
    pos1 = enc._pos_tokens[0]
    pos2 = enc._pos_tokens[-1]
    hello = enc.encode("Hello") + [pos1] + enc.encode("world") + [pos2]
    test = enc.decode(hello)
    assert test == "Hello⪦AAAAA⪧world⪦ADDDD⪧", test


def test_rev50000_derivatives(enc_name):
    print(termcolor.colored("\ntesting \"%s\"" % enc_name, "green"))
    enc = RefactEncoding(enc_name)
    msg = "I can feel the magic, can you?\n\nПривет мир!!!"
    toks = enc.encode(msg)
    print("encode", toks)
    assert toks == [40, 460, 1254, 262, 5536, 11, 460, 345, 30, 198, 198, 140, 253, 21169, 18849, 38857, 16843, 20375, 12466, 120, 18849, 21169, 10185], toks
    # Compare to https://beta.openai.com/tokenizer?view=bpe
    msg2 = enc.decode(toks)
    print("decode", msg2)
    assert msg2 == msg

    msg = "Hello world 1<|endoftext|>Hello world 2"
    toks = enc.encode(msg)
    assert toks == [15496, 995, 352, 27, 91, 437, 1659, 5239, 91, 29, 15496, 995, 362]
    assert enc.EOT == 50256
    toks.append(enc.EOT)
    print("encode", toks)
    msg2 = enc.decode(toks)
    print("decode", msg2)
    assert msg2 == msg + "<|endoftext|>"

    test_long_text(enc)
    if enc_name == "openai_programming_v2":
        test_position_tokens(enc)


def test_cl100k(enc_name):
    print(termcolor.colored("\ntesting \"%s\"" % enc_name, "green"))
    enc = RefactEncoding(enc_name)
    msg = "I can feel the magic, can you?\n\nПривет мир!!!"
    toks = enc.encode(msg)
    print("encode", toks)
    assert toks == [40, 649, 2733, 279, 11204, 11, 649, 499, 30, 198, 198, 54745, 28089, 8341, 11562, 78746, 12340], toks
    msg2 = enc.decode(toks)
    print("decode", msg2)
    assert msg2 == msg
    test_long_text(enc)


def test_llama(enc_name):
    print(termcolor.colored("\ntesting \"%s\"" % enc_name, "green"))
    enc = RefactEncoding(enc_name)
    msg = "I can feel the magic, can you?\n\nПривет мир!!!"
    toks = enc.encode(msg)
    print("encode", toks)
    # Note: token "1" is bos
    assert toks == [1, 306, 508, 4459, 278, 15709, 29892, 508, 366, 29973, 13, 13, 30013, 641, 7616, 4157, 29927, 21004], toks
    msg2 = enc.decode(toks)
    print("decode", msg2)
    assert msg2 == msg
    test_long_text(enc)


def test_bigcode(enc_name):
    print(termcolor.colored("\ntesting \"%s\"" % enc_name, "green"))
    enc = RefactEncoding(enc_name)
    msg = "I can feel the magic, can you?\n\nПривет мир!!!"
    toks = enc.encode(msg)
    print("encode", toks)
    assert toks == [59, 883, 10871, 322, 17281, 30, 883, 844, 49, 203, 203, 37505, 13840, 5046, 6963, 34810, 20863], toks
    msg2 = enc.decode(toks)
    print("decode", msg2)
    assert msg2 == msg
    test_long_text(enc)


if __name__ == "__main__":
    test_rev50000_derivatives("openai_reversible50000")
    test_rev50000_derivatives("openai_programming_v2")
    test_cl100k("openai_cl100k")
    test_llama("llama")
    test_bigcode("bigcode_largemodel")
