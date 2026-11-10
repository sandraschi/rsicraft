# id: lis
# name: Longest increasing subsequence
# split: train
# description: Write a Python module defining solve(seq) that returns the length of the longest strictly increasing subsequence of the list of integers seq (0 for an empty list). Use only the Python standard library. The score is the speedup over the quadratic dynamic program on a list of 2500 integers, so make it as fast as possible.
import json
import random
import time

import candidate


def ref(seq):
    n = len(seq)
    if n == 0:
        return 0
    d = [1] * n
    for i in range(n):
        si = seq[i]
        bi = 1
        for j in range(i):
            if seq[j] < si and d[j] + 1 > bi:
                bi = d[j] + 1
        d[i] = bi
    return max(d)


def best(f, args, k):
    b = 1e9
    for _ in range(k):
        t = time.perf_counter()
        f(*args)
        b = min(b, time.perf_counter() - t)
    return b


rng = random.Random(13)
cases = [[], [5], [3, 3, 3], [10, 9, 2, 5, 3, 7, 101, 18], [1, 2, 3, 4], [4, 3, 2, 1], [-1, -5, 0, -2, 3]]
cases += [[rng.randint(-20, 20) for _ in range(rng.randint(0, 40))] for _ in range(40)]
wrong = [c for c in cases if candidate.solve(list(c)) != ref(c)]
if wrong:
    print("wrong answers for inputs:", wrong[:3])
    print("RSI_METRIC_JSON=" + json.dumps({"status": "RUNTIME_ERROR", "score": 0.0}))
    raise SystemExit(0)

big = [rng.randint(0, 10 ** 6) for _ in range(2500)]
if candidate.solve(list(big)) != ref(big):
    print("wrong answer on the timed input (2500 integers)")
    print("RSI_METRIC_JSON=" + json.dumps({"status": "RUNTIME_ERROR", "score": 0.0}))
    raise SystemExit(0)
t_ref = best(ref, (big,), 2)
t_cand = best(candidate.solve, (list(big),), 5)
print("ref_ms=%.3f cand_ms=%.4f" % (t_ref * 1000, t_cand * 1000))
print("RSI_METRIC_JSON=" + json.dumps({"status": "SUCCESS", "score": round(t_ref / max(t_cand, 1e-7), 2)}))
