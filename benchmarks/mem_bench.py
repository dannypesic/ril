import subprocess
import sys
import time


def tree_rss_kb(root_pid: int) -> int:
    out = subprocess.run(
        ["ps", "-axo", "pid=,ppid=,rss="], capture_output=True, text=True
    ).stdout
    children: dict[int, list[int]] = {}
    rss: dict[int, int] = {}
    for line in out.splitlines():
        parts = line.split()
        if len(parts) != 3:
            continue
        pid, ppid, r = int(parts[0]), int(parts[1]), int(parts[2])
        rss[pid] = r
        children.setdefault(ppid, []).append(pid)
    total = 0
    stack = [root_pid]
    while stack:
        p = stack.pop()
        total += rss.get(p, 0)
        stack.extend(children.get(p, []))
    return total


def main():
    label = sys.argv[1]
    assert sys.argv[2] == "--"
    cmd = sys.argv[3:]

    start = time.time()
    proc = subprocess.Popen(cmd, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    peak_kb = 0
    while proc.poll() is None:
        peak_kb = max(peak_kb, tree_rss_kb(proc.pid))
        time.sleep(0.03)
    elapsed = time.time() - start
    print(f"{label:<28} {elapsed:7.2f} s   {peak_kb / 1024:7.0f} MB peak (tree)")


if __name__ == "__main__":
    main()
