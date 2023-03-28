"""
1. systemd_blame for baseline and update:

    2.539s cloud-init-local.service     

2. find for last -> finish: init-local -> previous line: 

2023-03-27 15:52:22,568 - util.py[DEBUG]: cloud-init mode 'init' took 0.333 seconds (0.34)
2023-03-27 15:52:22,569 - handlers.py[DEBUG]: finish: init-local: SUCCESS: searching for local datasources

3. Calculate netplan generate time (last occurrence):

2023-03-27 15:49:06,560 - subp.py[DEBUG]: Running command ['netplan', 'generate'] with allowed return codes [0] (shell=False, capture=True)
2023-03-27 15:49:06,890 - subp.py[DEBUG]: Running command ['udevadm', 'test-builtin', 'net_setup_link', '/sys/class/net/ens4'] with allowed return codes [0] (shell=False, capture=True)
"""
from pathlib import Path
import tarfile
import datetime as dt
import pandas as pd

INDIR = Path("./tmp/output")


def extract_systemd_blame(txt: bytes):
    """
    Extracts time from:

      2.539s cloud-init-local.service     
    """
    for line in txt.splitlines():
        if b"cloud-init-local.service" in line:
            return float(line.strip().split(b' ')[0][:-1])
    return None


def extract_cloud_init_log(txt: bytes):
    lines = list(reversed(txt.splitlines()))

    def extract_local_time():
        """
        2023-03-27 15:52:22,568 - util.py[DEBUG]: cloud-init mode 'init' took 0.333 seconds (0.34)
        2023-03-27 15:52:22,569 - handlers.py[DEBUG]: finish: init-local: SUCCESS: searching for local datasources
        """
        for idx, line in enumerate(lines):
            if b"finish: init-local: SUCCESS" in line:
                break
        return float(lines[idx + 1].split(b" ")[-3])
    
    local_time = extract_local_time()

    def extract_netplan_time():
        """
        Calculates elapsed time in seconds for `netplan generate`

        2023-03-27 15:49:06,560 - subp.py[DEBUG]: Running command ['netplan', 'generate'] with allowed return codes [0] (shell=False, capture=True)
        2023-03-27 15:49:06,890 - subp.py[DEBUG]: Running command ['udevadm', 'test-builtin', 'net_setup_link', '/sys/class/net/ens4'] with allowed return codes [0] (shell=False, capture=True)
        """
        for idx, line in enumerate(lines):
            if b"Running command ['netplan', 'generate']" in line:
                break
        
        ex_time = lambda ll: dt.datetime.strptime(ll.split(b" ")[1].decode(), "%H:%M:%S,%f")
        t_0 = ex_time(line)
        t_1 = ex_time(lines[idx - 1])  # substraction as the order is reversed

        return (t_1 - t_0).total_seconds()
    
    netplan_time = extract_netplan_time()
    return local_time, netplan_time


def main():
    data = []
    for release_path in INDIR.glob("*/"):
        release = release_path.name
        for boot_info_path in release_path.glob("*"):
            timestamp, base_or_update, boot_number, *_ = str(boot_info_path.name).split("-")
            row = dict()
            row["release"] = release
            row["timestamp"] = timestamp
            row["base_or_update"] = base_or_update
            with tarfile.open(boot_info_path) as boot_info_tar:
                systemd_blame = boot_info_tar.extractfile("./systemd_blame.txt").read()
                systemd_local_time = extract_systemd_blame(systemd_blame)
                row["systemd_local_time"] = systemd_local_time
                # TODO: store local_time
                if "1s" == boot_number:
                    continue
                print(boot_info_path)
                with tarfile.open(fileobj=boot_info_tar.extractfile("./cloud-init.tar.gz")) as cloud_init_logs:
                    cloud_init_log = cloud_init_logs.extractfile("cloud-init-logs-2023-03-27/cloud-init.log").read()
                    local_time, netplan_time = extract_cloud_init_log(cloud_init_log)
                    row["local_time"] = local_time
                    row["netplan_time"] = netplan_time
            data.append(row)

    df = pd.DataFrame(data)
    df.to_csv(INDIR / "data.csv", index=False)

if __name__ == "__main__":
    main()
