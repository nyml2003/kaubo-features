def pipeline():
    t = 0
    for x in range(1, 100001):
        if x % 2:
            m = x * 3
            if m % 7 == 0: t += m
    return t

print(pipeline())
