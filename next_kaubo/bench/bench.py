import time; import sys

def fib_iter(n):
    if n <= 1: return n
    a, b = 0, 1
    for _ in range(2, n + 1): a, b = b, a + b
    return b

def mandelbrot():
    w, h = 500, 500; max_iter = 50
    xmin, xmax = -2.0, 1.0; ymin, ymax = -1.5, 1.5
    dx = (xmax - xmin) / w; dy = (ymax - ymin) / h
    for py in range(h):
        y0 = ymin + py * dy
        for px in range(w):
            x0 = xmin + px * dx; x, y = 0.0, 0.0
            for _ in range(max_iter):
                if x*x + y*y > 4.0: break
                y = 2.0*x*y + y0; x = x*x - y*y + x0

def sieve(n):
    count = 0
    for p in range(2, n + 1):
        ip = True; d = 2
        while d * d <= p:
            if p % d == 0: ip = False; break
            d += 1
        if ip: count += 1
    return count

def pipeline():
    total = 0
    for x in range(1, 100001):
        if x % 2 != 0:
            t = x * 3
            if t % 7 == 0: total += t
    return total

print("=== CPython", sys.version.split()[0], "===")
for _ in range(3): s=time.perf_counter(); r=fib_iter(35); print(f"fib {r} {((time.perf_counter()-s)*1000):.0f}ms")
for _ in range(3): s=time.perf_counter(); mandelbrot(); print(f"mand {((time.perf_counter()-s)*1000):.0f}ms")
for _ in range(3): s=time.perf_counter(); r=sieve(100000); print(f"sie {r} {((time.perf_counter()-s)*1000):.0f}ms")
for _ in range(3): s=time.perf_counter(); r=pipeline(); print(f"pip {r} {((time.perf_counter()-s)*1000):.0f}ms")
