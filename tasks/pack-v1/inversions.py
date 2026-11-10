# id: inversions
# name: Count inversions
# split: heldout
# description: Write a Python module defining solve(seq) that returns the number of inversions in the list of integers seq, that is the number of index pairs i < j with seq[i] > seq[j] (equal elements are not inversions). Do not modify the input list. Use only the Python standard library. The score is the speedup over the quadratic double loop on a list of 3000 integers, so make it as fast as possible.
import json
import random
import time

import candidate


def ref(seq):
    n = len(seq)
    c = 0
    for i in range(n):
        si = seq[i]
        for j in range(i + 1, n):
            if si > seq[j]:
                c += 1
    return c


def best(f, args, k):
    b = 1e9
    for _ in range(k):
        t = time.perf_counter()
        f(*args)
        b = min(b, time.perf_counter() - t)
    return b


rng = random.Random(17)
cases = [[], [1], [1, 2, 3], [3, 2, 1], [2, 2, 2], [1, 3, 2, 3, 1], [-1, -2, 0]]
cases += [[rng.randint(-10, 10) for _ in range(rng.randint(0, 40))] for _ in range(40)]
wrong = [c for c in cases if candidate.solve(list(c)) != ref(c)]
if wrong:
    print("wrong answers for inputs:", wrong[:3])
    print("RSI_METRIC_JSON=" + json.dumps({"status": "RUNTIME_ERROR", "score": 0.0}))
    raise SystemExit(0)

big = [rng.randint(0, 10 ** 6) for _ in range(3000)]
if candidate.solve(list(big)) != ref(big):
    print("wrong answer on the timed input (3000 integers)")
    print("RSI_METRIC_JSON=" + json.dumps({"status": "RUNTIME_ERROR", "score": 0.0}))
    raise SystemExit(0)
t_ref = best(ref, (big,), 2)
t_cand = best(candidate.solve, (list(big),), 5)
print("ref_ms=%.3f cand_ms=%.4f" % (t_ref * 1000, t_cand * 1000))
print("RSI_METRIC_JSON=" + json.dumps({"status": "SUCCESS", "score": round(t_ref / max(t_cand, 1e-7), 2)}))
