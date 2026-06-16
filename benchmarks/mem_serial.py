import sys
import pandas as pd

inp, out = sys.argv[1], sys.argv[2]
df = pd.read_csv(inp)
df["score"] = df["value"] * 2.0
df.to_csv(out, index=False)
