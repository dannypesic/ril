import sys
import pandas as pd
from workload import heavy

inp, out = sys.argv[1], sys.argv[2]

df = pd.read_csv(inp)
df["score"] = [heavy(v) for v in df["value"]]
df.to_csv(out, index=False)
