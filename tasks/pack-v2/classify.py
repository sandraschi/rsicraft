# id: classify
# name: Tabular classification without libraries
# split: heldout
# description: Write a Python module defining solve(train_X, train_y, test_X). train_X and test_X are lists of rows, each row a list of 8 floats in [-1, 1]; train_y is a list of labels, each 0 or 1. Return a list with one predicted label (0 or 1) per row of test_X. The label follows an unknown nonlinear rule over some of the features, with a little label noise; some features are irrelevant. There are 400 training rows and 400 test rows per dataset, and each call must return within 20 seconds or it scores zero. Use only the Python standard library (no numpy, no scikit-learn). Score is 100 * (accuracy - 0.5) / 0.5 on the test rows (below-chance counts as 0), averaged over 2 datasets; higher is better.
import json
import math
import random
import time

import candidate

LIMIT = 20.0


def make(seed, n, noise):
    rng = random.Random(seed)
    X, y = [], []
    for _ in range(n):
        r = [rng.uniform(-1, 1) for _ in range(8)]
        label = 1 if math.sin(3 * r[0]) + r[1] * r[1] - r[2] * r[3] > 0.3 else 0
        if rng.random() < noise:
            label = 1 - label
        X.append(r)
        y.append(label)
    return X, y


scores = []
notes = []
for seed in (41, 42):
    trX, trY = make(seed, 400, 0.05)
    teX, teY = make(seed + 1000, 400, 0.0)
    t = time.perf_counter()
    out = candidate.solve([list(r) for r in trX], list(trY), [list(r) for r in teX])
    dt = time.perf_counter() - t
    pred = list(out)
    if len(pred) != len(teY) or any(p not in (0, 1) for p in pred):
        print("invalid predictions for dataset", seed, ": need", len(teY), "labels that are 0 or 1")
        print("RSI_METRIC_JSON=" + json.dumps({"status": "RUNTIME_ERROR", "score": 0.0}))
        raise SystemExit(0)
    if dt > LIMIT:
        notes.append("dataset %d took %.1fs (limit %.0fs): scored 0" % (seed, dt, LIMIT))
        scores.append(0.0)
        continue
    acc = sum(1 for a, b in zip(teY, pred) if a == b) / len(teY)
    scores.append(max(0.0, (acc - 0.5) / 0.5))
    notes.append("dataset %d accuracy %.4f in %.1fs" % (seed, acc, dt))

print("\n".join(notes))
print("RSI_METRIC_JSON=" + json.dumps({"status": "SUCCESS", "score": round(100 * sum(scores) / len(scores), 2)}))
