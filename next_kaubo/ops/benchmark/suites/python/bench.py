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

def list_push(n=100000):
    l = []
    for i in range(n):
        l.append(i)
    return len(l)

def string_concat(n=1000):
    s = "a"
    for _ in range(n):
        s += "b"
    return "ok"

def json_access(n=10000):
    d = {"value": 0}
    for _ in range(n):
        d = {"value": d["value"] + 1}
    return "ok"

def closure_call(n=100000):
    f = lambda x: x + 1
    for _ in range(n):
        f(0)
    return "ok"

def nested_loop(n=200):
    total = 0
    for i in range(n):
        for j in range(n):
            total += i * j
    return total

def fact_loop(n=100):
    def fac(m):
        r = 1
        for i in range(1, m + 1):
            r *= i
        return r
    total = 0
    for x in range(1, n + 1):
        total += fac(x)
    return total

if __name__ == "__main__":
    import time, sys
    if len(sys.argv) > 1:
        fn = sys.argv[1]
        if fn == "fib": print(fib_iter(40))
        elif fn == "mandelbrot": print(mandelbrot())
        elif fn == "sieve": print(sieve(100000))
        elif fn == "pipeline": print(pipeline())
        elif fn == "list_push": print(list_push())
        elif fn == "string_concat": print(string_concat())
        elif fn == "json_access": print(json_access())
        elif fn == "closure_call": print(closure_call())
    else:
        for name, fn in [("fib", lambda: fib_iter(40)), ("mandelbrot", mandelbrot),
                         ("sieve", lambda: sieve(100000)), ("pipeline", pipeline),
                         ("list_push", list_push), ("string_concat", string_concat),
                         ("json_access", json_access), ("closure_call", closure_call)]:
            print(fn())
