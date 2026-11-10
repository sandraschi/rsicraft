# id: primes
# name: Prime count
# split: train
# description: Write a Python module defining solve(n) that returns the number of prime numbers strictly less than n. It must be correct for every integer n >= 0. Use only the Python standard library. The score is the speedup over naive trial division at n = 60000, so make it as fast as possible.
import json
import time

import candidate


def ref(n):
    c = 0
    for i in range(2, n):
        j = 2
        p = True
        while j * j <= i:
            if i % j == 0:
                p = False
                break
            j += 1
        if p:
            c += 1
    return c


def best(f, args, k):
    b = 1e9
    for _ in range(k):
        t = time.perf_counter()
        f(*args)
        b = min(b, time.perf_counter() - t)
    return b


cases = [(0, 0), (1, 0), (2, 0), (3, 1), (10, 4), (100, 25), (1000, 168), (20000, 2262)]
wrong = [n for n, e in cases if candidate.solve(n) != e]
if wrong:
    print("wrong answers for n in", wrong[:5])
    print("RSI_METRIC_JSON=" + json.dumps({"status": "RUNTIME_ERROR", "score": 0.0}))
    raise SystemExit(0)

args = (60000,)
t_ref = best(ref, args, 2)
t_cand = best(candidate.solve, args, 7)
print("ref_ms=%.3f cand_ms=%.4f" % (t_ref * 1000, t_cand * 1000))
print("RSI_METRIC_JSON=" + json.dumps({"status": "SUCCESS", "score": round(t_ref / max(t_cand, 1e-7), 2)}))
