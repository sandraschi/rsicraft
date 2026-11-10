# id: edit-distance
# name: Edit distance
# split: train
# description: Write a Python module defining solve(a, b) that returns the Levenshtein edit distance between the strings a and b (insertions, deletions and substitutions each cost 1). It must be correct for all strings including empty ones. Use only the Python standard library. The score is the speedup over a plain full-matrix dynamic program on two strings of length 400, so make it as fast as possible.
import json
import random
import time

import candidate


def ref(a, b):
    n, m = len(a), len(b)
    d = [[0] * (m + 1) for _ in range(n + 1)]
    for i in range(n + 1):
        d[i][0] = i
    for j in range(m + 1):
        d[0][j] = j
    for i in range(1, n + 1):
        for j in range(1, m + 1):
            cost = 0 if a[i - 1] == b[j - 1] else 1
            d[i][j] = min(d[i - 1][j] + 1, d[i][j - 1] + 1, d[i - 1][j - 1] + cost)
    return d[n][m]


def best(f, args, k):
    b = 1e9
    for _ in range(k):
        t = time.perf_counter()
        f(*args)
        b = min(b, time.perf_counter() - t)
    return b


rng = random.Random(7)


def rand_str(n):
    return "".join(rng.choice("abcd") for _ in range(n))


cases = [("", ""), ("a", ""), ("", "abc"), ("kitten", "sitting"), ("flaw", "lawn"), ("same", "same")]
cases += [(rand_str(rng.randint(0, 30)), rand_str(rng.randint(0, 30))) for _ in range(40)]
wrong = [(a, b) for a, b in cases if candidate.solve(a, b) != ref(a, b)]
if wrong:
    print("wrong answers for cases:", wrong[:3])
    print("RSI_METRIC_JSON=" + json.dumps({"status": "RUNTIME_ERROR", "score": 0.0}))
    raise SystemExit(0)

args = (rand_str(400), rand_str(400))
if candidate.solve(*args) != ref(*args):
    print("wrong answer on the timed input (two strings of length 400)")
    print("RSI_METRIC_JSON=" + json.dumps({"status": "RUNTIME_ERROR", "score": 0.0}))
    raise SystemExit(0)
t_ref = best(ref, args, 2)
t_cand = best(candidate.solve, args, 5)
print("ref_ms=%.3f cand_ms=%.4f" % (t_ref * 1000, t_cand * 1000))
print("RSI_METRIC_JSON=" + json.dumps({"status": "SUCCESS", "score": round(t_ref / max(t_cand, 1e-7), 2)}))
