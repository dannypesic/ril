import sys
import multiprocessing as mp
import pandas as pd
from workload import heavy

inp, out = sys.argv[1], sys.argv[2]

if __name__ == "__main__":
    df = pd.read_csv(inp)
    with mp.Pool(mp.cpu_count()) as pool:
        df["score"] = pool.map(heavy, df["value"].tolist(), chunksize=10_000)
    df.to_csv(out, index=False)
