def loop(n):
    t = 0
    for i in range(n):
        for j in range(n): t += i * j
    return t

print(loop(200))
