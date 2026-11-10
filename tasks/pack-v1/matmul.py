# id: matmul
# name: Matrix multiply
# split: train
# description: Write a Python module defining solve(A, B) that returns the matrix product of A and B. Matrices are lists of rows (lists of ints); A is n x m and B is m x p, not necessarily square. Return a list of lists. Use only the Python standard library (no numpy). The score is the speedup over a plain triple loop on two 70 x 70 matrices, so make it as fast as possible.
import json
import random
import time

import candidate


def ref(A, B):
    n, m, p = len(A), len(B), len(B[0])
    C = [[0] * p for _ in range(n)]
    for i in range(n):
        for j in range(p):
            s = 0
            for k in range(m):
                s += A[i][k] * B[k][j]
            C[i][j] = s
    return C


def best(f, args, k):
    b = 1e9
    for _ in range(k):
        t = time.perf_counter()
        f(*args)
        b = min(b, time.perf_counter() - t)
    return b


rng = random.Random(11)


def rand_mat(n, m):
    return [[rng.randint(-9, 9) for _ in range(m)] for _ in range(n)]


def same(x, y):
    return [list(r) for r in x] == y


shapes = [(1, 1, 1), (2, 3, 4), (3, 1, 2), (5, 5, 5), (4, 7, 3), (8, 2, 6)]
wrong = []
for n, m, p in shapes:
    A, B = rand_mat(n, m), rand_mat(m, p)
    if not same(candidate.solve(A, B), ref(A, B)):
        wrong.append((n, m, p))
if wrong:
    print("wrong product for shapes (n, m, p):", wrong[:5])
    print("RSI_METRIC_JSON=" + json.dumps({"status": "RUNTIME_ERROR", "score": 0.0}))
    raise SystemExit(0)

args = (rand_mat(70, 70), rand_mat(70, 70))
if not same(candidate.solve(*args), ref(*args)):
    print("wrong product on the timed 70 x 70 input")
    print("RSI_METRIC_JSON=" + json.dumps({"status": "RUNTIME_ERROR", "score": 0.0}))
    raise SystemExit(0)
t_ref = best(ref, args, 2)
t_cand = best(candidate.solve, args, 5)
print("ref_ms=%.3f cand_ms=%.4f" % (t_ref * 1000, t_cand * 1000))
print("RSI_METRIC_JSON=" + json.dumps({"status": "SUCCESS", "score": round(t_ref / max(t_cand, 1e-7), 2)}))
