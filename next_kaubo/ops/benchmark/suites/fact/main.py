def fact(n):
    t = 0
    for x in range(1, n + 1):
        r = 1
        for i in range(1, x + 1): r *= i
        t += r
    return t

print(fact(12))
