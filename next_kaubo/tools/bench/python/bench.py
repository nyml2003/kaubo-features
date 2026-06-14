"""Kaubo Benchmark — Python reference implementation"""

def fib_iter(n):
    if n <= 1: return n
    a, b = 0, 1
    for _ in range(2, n + 1): a, b = b, a + b
    return b

def mandelbrot():
    w, h, max_iter = 500, 500, 50
    xmin, xmax = -2.0, 1.0; ymin, ymax = -1.5, 1.5
    dx = (xmax - xmin) / w; dy = (ymax - ymin) / h
    for py in range(h):
        y0 = ymin + py * dy
        for px in range(w):
            x0 = xmin + px * dx; x, y = 0.0, 0.0
            for _ in range(max_iter):
                if x*x + y*y > 4.0: break
                y = 2.0*x*y + y0; x = x*x - y*y + x0
    return "ok"

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

if __name__ == "__main__":
    import time, sys
    if len(sys.argv) > 1:
        fn = sys.argv[1]
        if fn == "fib": print(fib_iter(40))
        elif fn == "mandelbrot": print(mandelbrot())
        elif fn == "sieve": print(sieve(100000))
        elif fn == "pipeline": print(pipeline())
    else:
        print(fib_iter(40))
        print(mandelbrot())
        print(sieve(100000))
        print(pipeline())
