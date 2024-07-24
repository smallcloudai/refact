import { describe, test, vi, expect } from "vitest";
import { render } from "../../utils/test-utils";
import { DiffContent } from "./DiffContent";

const STUB_DIFFS_1 = [
  {
    file_name:
      "/Users/marc/Projects/refact-lsp/tests/emergency_frog_situation/holiday.py",
    file_action: "edit",
    line1: 17,
    line2: 19,
    lines_remove: "    frog1.jump()\n    frog2.jump()\n",
    lines_add: "    bird1.jump()\n    bird2.jump()\n",
  },
  {
    file_name:
      "/Users/marc/Projects/refact-lsp/tests/emergency_frog_situation/holiday.py",
    file_action: "edit",
    line1: 21,
    line2: 23,
    lines_remove: "    frog1.jump()\n    frog2.jump()\n",
    lines_add: "    bird1.jump()\n    bird2.jump()\n",
  },
];

describe("diff content", () => {
  test("apply all", async () => {
    const appliedChunks = {
      fetching: false,
      error: null,
      diff_id: "call_3odUG8bPn1gER3DSOOcVizZS",
      state: [],
      applied_chunks: [false, false],
      can_apply: [true, true],
    };

    const onSumbitSpy = vi.fn();
    const { user, ...app } = render(
      <DiffContent
        diffs={STUB_DIFFS_1}
        appliedChunks={appliedChunks}
        onSubmit={onSumbitSpy}
      />,
    );

    // eslint-disable-next-line @typescript-eslint/no-non-null-assertion
    await user.click(app.container.querySelector('[type="button"]')!);
    const btn = app.getByText(/Apply all/i);
    await user.click(btn);
    expect(onSumbitSpy).toHaveBeenCalledWith([true, true]);
  });

  test("unapply all", async () => {
    const appliedChunks = {
      fetching: false,
      error: null,
      diff_id: "call_3odUG8bPn1gER3DSOOcVizZS",
      state: [],
      applied_chunks: [true, true],
      can_apply: [true, true],
    };

    const onSumbitSpy = vi.fn();
    const { user, ...app } = render(
      <DiffContent
        diffs={STUB_DIFFS_1}
        appliedChunks={appliedChunks}
        onSubmit={onSumbitSpy}
      />,
    );

    // eslint-disable-next-line @typescript-eslint/no-non-null-assertion
    await user.click(app.container.querySelector('[type="button"]')!);
    const btn = app.getByText(/unapply all/i);
    await user.click(btn);
    expect(onSumbitSpy).toHaveBeenCalledWith([true, true]);
  });

  test("disable apply all", async () => {
    const appliedChunks = {
      fetching: false,
      error: null,
      diff_id: "call_3odUG8bPn1gER3DSOOcVizZS",
      state: [],
      applied_chunks: [false, false],
      can_apply: [true, false],
    };

    const onSumbitSpy = vi.fn();
    const { user, ...app } = render(
      <DiffContent
        diffs={STUB_DIFFS_1}
        appliedChunks={appliedChunks}
        onSubmit={onSumbitSpy}
      />,
    );

    // eslint-disable-next-line @typescript-eslint/no-non-null-assertion
    await user.click(app.container.querySelector('[type="button"]')!);
    const btn = app.getByText(/apply all/i) as HTMLButtonElement;
    expect(btn.disabled).toBe(true);
    await user.click(btn);
    expect(onSumbitSpy).not.toHaveBeenCalled();
  });
});
