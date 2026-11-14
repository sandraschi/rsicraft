# id: tsp
# name: Euclidean TSP heuristic
# split: train
# description: Write a Python module defining solve(points) where points is a list of (x, y) float tuples. Return a tour as a list containing every index 0..len(points)-1 exactly once; the tour is closed (the last point connects back to the first). The goal is the shortest possible total Euclidean length. There are 150 points per instance and each call must return within 3 seconds or it scores zero. Use only the Python standard library. Score is 100 times the nearest-neighbour tour length divided by your tour length, averaged over 4 instances, so 100 means "as good as nearest neighbour" and higher is better.
import json
import math
import random
import time

import candidate

LIMIT = 3.0


def tour_len(pts, order):
    return sum(math.dist(pts[order[i]], pts[order[(i + 1) % len(order)]]) for i in range(len(order)))


def nearest_neighbour(pts):
    left = set(range(1, len(pts)))
    order = [0]
    while left:
        cur = pts[order[-1]]
        nxt = min(left, key=lambda j: math.dist(cur, pts[j]))
        left.remove(nxt)
        order.append(nxt)
    return order


ratios = []
notes = []
for seed in (101, 202, 303, 404):
    rng = random.Random(seed)
    pts = [(rng.random(), rng.random()) for _ in range(150)]
    t = time.perf_counter()
    out = candidate.solve(list(pts))
    dt = time.perf_counter() - t
    order = list(out)
    if sorted(order) != list(range(len(pts))):
        print("invalid tour for seed", seed, ": not a permutation of all indices")
        print("RSI_METRIC_JSON=" + json.dumps({"status": "RUNTIME_ERROR", "score": 0.0}))
        raise SystemExit(0)
    if dt > LIMIT:
        notes.append("seed %d took %.2fs (limit %.1fs): scored 0" % (seed, dt, LIMIT))
        ratios.append(0.0)
        continue
    ratios.append(tour_len(pts, nearest_neighbour(pts)) / tour_len(pts, order))
    notes.append("seed %d ratio %.4f in %.2fs" % (seed, ratios[-1], dt))

print("\n".join(notes))
print("RSI_METRIC_JSON=" + json.dumps({"status": "SUCCESS", "score": round(100 * sum(ratios) / len(ratios), 2)}))
