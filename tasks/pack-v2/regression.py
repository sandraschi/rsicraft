# id: regression
# name: Tabular regression without libraries
# split: train
# description: Write a Python module defining solve(train_X, train_y, test_X). train_X and test_X are lists of rows, each row a list of 10 floats in [0, 1]; train_y is a list of floats. Return a list with one predicted float per row of test_X. The target is an unknown noisy nonlinear function of some of the features; some features are irrelevant. There are 400 training rows and 400 test rows per dataset, and each call must return within 20 seconds or it scores zero. Use only the Python standard library (no numpy, no scikit-learn). Score is 100 times the R-squared on the test rows (negative values count as 0), averaged over 2 datasets; higher is better.
import json
import math
import random
import time

import candidate

LIMIT = 20.0


def make(seed, n):
    rng = random.Random(seed)
    X, y = [], []
    for _ in range(n):
        r = [rng.random() for _ in range(10)]
        v = 10 * math.sin(math.pi * r[0] * r[1]) + 20 * (r[2] - 0.5) ** 2 + 10 * r[3] + 5 * r[4] + rng.gauss(0, 1)
        X.append(r)
        y.append(v)
    return X, y


def r2(y, p):
    m = sum(y) / len(y)
    ss_tot = sum((v - m) ** 2 for v in y)
    ss_res = sum((v - q) ** 2 for v, q in zip(y, p))
    return 1 - ss_res / ss_tot


scores = []
notes = []
for seed in (11, 22):
    trX, trY = make(seed, 400)
    teX, teY = make(seed + 1000, 400)
    t = time.perf_counter()
    out = candidate.solve([list(r) for r in trX], list(trY), [list(r) for r in teX])
    dt = time.perf_counter() - t
    pred = [float(v) for v in out]
    if len(pred) != len(teY) or any(math.isnan(v) or math.isinf(v) for v in pred):
        print("invalid predictions for dataset", seed, ": need", len(teY), "finite floats, got", len(pred))
        print("RSI_METRIC_JSON=" + json.dumps({"status": "RUNTIME_ERROR", "score": 0.0}))
        raise SystemExit(0)
    if dt > LIMIT:
        notes.append("dataset %d took %.1fs (limit %.0fs): scored 0" % (seed, dt, LIMIT))
        scores.append(0.0)
        continue
    scores.append(max(0.0, r2(teY, pred)))
    notes.append("dataset %d R2 %.4f in %.1fs" % (seed, scores[-1], dt))

print("\n".join(notes))
print("RSI_METRIC_JSON=" + json.dumps({"status": "SUCCESS", "score": round(100 * sum(scores) / len(scores), 2)}))
