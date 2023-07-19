import collections
import sys
import re
import copy
import os
import jsonlines
from typing import List, Dict


def smooth(y, radius):
    import numpy as np
    kernel = np.zeros(2 * radius + 1)
    kernel[: radius + 1] = np.linspace(0, 1, radius + 2)[1:]
    assert kernel.size % 2 == 1
    radius = kernel.size // 2
    EPS = 1e-20
    return (
        np.correlate(y, kernel, mode="full")
        / (np.correlate(np.ones_like(y), kernel, mode="full") + EPS)
    )[radius:-radius]


def plot(
    xaxis: str,
    x0: float,
    x1: float,
    yaxis: str,
    jdict: Dict[str, List[Dict[str, float]]],
    colors: List[str],
):
    import numpy as np
    import matplotlib
    matplotlib.use('Agg')
    import matplotlib.pyplot as plt
    import io

    xs = collections.defaultdict(list)
    ys = collections.defaultdict(list)
    smoo = 0
    y0 = -1e10
    y1 = -1e10
    logscale = False

    m = re.fullmatch("(.*)\[([-0-9.e]+),([-0-9.e]+)\](.*)", yaxis)
    if m:
        options = [x.strip() for x in m.group(4).split(",")]
        yaxis = m.group(1)
        y0 = float(m.group(2))
        y1 = float(m.group(3))
    else:
        options = [x.strip() for x in yaxis.split(",")]
        yaxis = options[0]
    options = options[1:]
    for o in options:
        m = re.fullmatch("smooth={0,1}([0-9]+)", o)
        if m:
            smoo = int(m.group(1))
        elif o == "log":
            logscale = True
        else:
            raise ValueError("Invalid option \"%s\"" % o)
    for f in jdict:
        for j in jdict[f]:
            if xaxis in j and yaxis in j:
                xs[f].append(j[xaxis])
                ys[f].append(j[yaxis])

    # No errors should happen after this point
    smoo_ys = collections.defaultdict(int)
    x0auto = +1e10
    x1auto = -1e10
    y0auto = +1e10
    y1auto = -1e10
    for f in jdict:
        if smoo > 0 and len(ys[f]) > 0:
            smoo_ys[f] = smooth(ys[f], radius=smoo)
        if len(xs[f]) > 0:
            x0auto = min(x0auto, min(xs[f]))
            x1auto = max(x1auto, max(xs[f]))
        ys_finite = [y for y in ys[f] if np.isfinite(y)]
        if len(ys_finite) > 0:
            y0auto = min(y0auto, min(ys_finite))
            y1auto = max(y1auto, max(ys_finite))
    x0 = x0 if x0 != -1e10 else x0auto
    x1 = x1 if x1 != -1e10 else x1auto
    y0 = y0 if y0 != -1e10 else y0auto
    y1 = y1 if y1 != -1e10 else y1auto

    buf = io.BytesIO()
    plt.figure(figsize=(6, 3))
    plots_for_legend = []
    for i, f in enumerate(jdict.keys()):
        if len(xs[f]) == 0:
            continue
        if len(ys[f]) == 0:
            continue
        if f in smoo_ys and colors[i] is not None:
            plt.plot(xs[f], smoo_ys[f], color=colors[i])
            p = plt.plot(xs[f], ys[f], color=colors[i], alpha=0.2)
        elif f in smoo_ys and colors[i] is None:
            p = plt.plot(xs[f], smoo_ys[f])
        else:
            p = plt.plot(xs[f], ys[f], color=colors[i])
        plots_for_legend.append(p[0])
    plt.xlim(x0, x1)
    plt.ylim(y0, y1)
    if logscale:
        plt.yscale("log")
    plt.grid(which="both", alpha=0.2)
    plt.title(yaxis, loc="right")
    plt.legend(plots_for_legend, [k for k in jdict.keys()], loc="upper right")
    plt.savefig(buf, format='svg')
    plt.close('all')
    buf.seek(0)
    return buf


if __name__ == "__main__":
    jdict = {}
    jdict["test"] = []
    jdict["train"] = list(jsonlines.open(sys.argv[1]))
    for line in jdict["train"]:
        line = copy.deepcopy(line)
        if "test_loss" in line:
            line["loss"] = line["test_loss"]
        jdict["test"].append(line)
    if len(jdict["test"]) == 0:
        jdict.pop("test")
    buf = plot(
        "iteration",
        0,
        int(sys.argv[2]),
        "loss[0,2.6]",  #,smooth5
        jdict,
        ["#ff0000", "#880000"],
    )
    # save
    with open("progress.svg.tmp", "wb") as f:
        f.write(buf.getvalue())
    os.rename("progress.svg.tmp", "progress.svg")
