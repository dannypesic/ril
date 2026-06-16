K = 400

def heavy(x: float) -> float:
    acc = 0.0
    for i in range(K):
        acc += ((x * 1.0000001) + i) ** 0.5 - (x / (i + 1.0))
    return acc
