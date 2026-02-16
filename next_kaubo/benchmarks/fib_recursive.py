# 斐波那契递归 - CPU密集型
import sys
sys.setrecursionlimit(2000)

def fib(n):
    if n <= 1:
        return n
    return fib(n - 1) + fib(n - 2)

# 测试 n=30 的递归深度
result = fib(30)
print(f"fib(30) = {result}")
