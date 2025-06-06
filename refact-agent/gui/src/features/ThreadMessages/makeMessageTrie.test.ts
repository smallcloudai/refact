import { expect, describe, test } from "vitest";
import { makeMessageTrie } from "./makeMessageTrie";
import { STUB_ALICE_MESSAGES } from "../../__fixtures__/message_lists";

describe("makeMessageTree", () => {
  test("stub data", () => {
    const result = makeMessageTrie(STUB_ALICE_MESSAGES);

    expect(result).toEqual({
      value: STUB_ALICE_MESSAGES[0],
      children: [
        {
          value: STUB_ALICE_MESSAGES[1],
          children: [{ value: STUB_ALICE_MESSAGES[2], children: [] }],
        },
      ],
    });
  });
});
