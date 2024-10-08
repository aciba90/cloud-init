# This file is part of cloud-init. See LICENSE file for license information.
import re

import pytest

from tests.integration_tests.instances import IntegrationInstance
from tests.integration_tests.util import verify_clean_boot, verify_clean_log

USER_DATA = """\
## template: jinja
#cloud-boothook
#!/bin/sh
# Error below will generate stderr
BOOTHOOK/0
echo BOOTHOOKstdout
echo "BOOTHOOK: {{ v1.instance_id }}: is called every boot." >> /boothook.txt
"""


@pytest.mark.user_data(USER_DATA)
def test_boothook_header_runs_part_per_instance(client: IntegrationInstance):
    """Test boothook handling creates a script that runs per-boot.
    Streams stderr and stdout are directed to /var/log/cloud-init-output.log.
    """
    instance_id = client.instance.execute("cloud-init query instance-id")
    RE_BOOTHOOK = f"BOOTHOOK: {instance_id}: is called every boot"
    log = client.read_from_file("/var/log/cloud-init.log")
    verify_clean_log(log)
    verify_clean_boot(client)
    output = client.read_from_file("/boothook.txt")
    assert 1 == len(re.findall(RE_BOOTHOOK, output))
    client.restart()
    output = client.read_from_file("/boothook.txt")
    assert 2 == len(re.findall(RE_BOOTHOOK, output))
    output_log = client.read_from_file("/var/log/cloud-init-output.log")
    expected_msgs = [
        "BOOTHOOKstdout",
        "boothooks/part-001: 3: BOOTHOOK/0: not found",
    ]
    for msg in expected_msgs:
        assert msg in output_log


def test_clean_reboot(client: IntegrationInstance):
    log = client.read_from_file("/var/log/cloud-init.log")
    verify_clean_log(log)
    verify_clean_boot(client)
    client.execute("rm -rv /etc/netplan/50-cloud-init.yaml", use_sudo=True)
    client.execute(
        "cloud-init clean --logs --machine-id -c all", use_sudo=True
    )

    client.restart()

    log = client.read_from_file("/var/log/cloud-init.log")
    verify_clean_log(log)
    verify_clean_boot(client)
