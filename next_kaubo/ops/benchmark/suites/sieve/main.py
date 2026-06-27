def sieve(n):
    c = 0
    for p in range(2, n + 1):
        ip = True; d = 2
        while d * d <= p:
            if p % d == 0: ip = False; break
            d += 1
        if ip: c += 1
    return c

print(sieve(100000))
