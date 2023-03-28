import pytest

from tests.integration_tests.instances import IntegrationInstance
from tests.integration_tests.util import verify_clean_log

USER_DATA = """\
#cloud-config

# apply network config on every boot
updates:
  network:
    when: ['boot']
"""

COLLECT_SCRIPT = """\
#!/bin/bash

set -eux

outdir=$(mktemp --directory)

systemd-analyze blame > "${outdir}/systemd_blame.txt"
systemd-analyze critical-chain > "${outdir}/systemd_critical_chain.txt"
systemd-analyze plot > "${outdir}/systemd_plot.svg"

cloud-init analyze blame > "${outdir}/cloud_init_blame.txt"
cloud-init analyze show > "${outdir}/cloud_init_show.txt"
cloud-init analyze boot > "${outdir}/cloud_init_boot.txt" | true
cloud-init analyze dump > "${outdir}/cloud_init_dump.txt"

cloud-init collect-logs --include-userdata --tarfile "${outdir}/cloud-init.tar.gz"

tar -C "$outdir" -caf /tmp/boot-analysis.tar.xz .
"""


def collect_boot_logs(client: IntegrationInstance, outname):
    client.write_to_file("/tmp/collect.sh", COLLECT_SCRIPT)
    assert client.execute("chmod +x /tmp/collect.sh").ok
    assert client.execute("/tmp/collect.sh").ok
    client.pull_file("/tmp/boot-analysis.tar.xz", outname)


def do_test(client: IntegrationInstance):
    log = client.read_from_file("/var/log/cloud-init.log")
    verify_clean_log(log)
    collect_boot_logs(client, "/tmp/1st-boot-analysis.tar.xz")

    # Second boot
    client.restart()
    log = client.read_from_file("/var/log/cloud-init.log")
    verify_clean_log(log)
    collect_boot_logs(client, "/tmp/2nd-boot-analysis.tar.xz")


@pytest.mark.gce
@pytest.mark.lxd_container
@pytest.mark.user_data(USER_DATA)
def test_network_update_on_reboot(client: IntegrationInstance):
    do_test(client)


@pytest.mark.gce
@pytest.mark.lxd_container
def test_network_baseline(client: IntegrationInstance):
    do_test(client)
