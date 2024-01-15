"""Networking-related tests."""
import contextlib

import pytest
import yaml

from cloudinit.subp import subp
from tests.integration_tests.clouds import IntegrationCloud
from tests.integration_tests.instances import IntegrationInstance
from tests.integration_tests.integration_settings import PLATFORM


def _add_dummy_bridge_to_netplan(client: IntegrationInstance):
    # Update netplan configuration to ensure it doesn't change on reboot
    netplan = yaml.safe_load(
        client.execute("cat /etc/netplan/50-cloud-init.yaml")
    )
    # Just a dummy bridge to do nothing
    try:
        netplan["network"]["bridges"]["dummy0"] = {"dhcp4": False}
    except KeyError:
        netplan["network"]["bridges"] = {"dummy0": {"dhcp4": False}}

    dumped_netplan = yaml.dump(netplan)
    client.write_to_file("/etc/netplan/50-cloud-init.yaml", dumped_netplan)


USER_DATA = """\
#cloud-config
updates:
  network:
    when: [boot]
"""


@pytest.mark.skipif(
    PLATFORM not in ("lxd_container", "lxd_vm"),
    reason=(
        f"{PLATFORM} could make nic changes in a reboot event invalidating"
        f" these tests."
    ),
)
@pytest.mark.user_data(USER_DATA)
class TestNetplanGenerateBehaviorOnReboot:
    def test_skip(self, client: IntegrationInstance):
        log = client.read_from_file("/var/log/cloud-init.log")
        assert "Applying network configuration" in log
        assert "Selected renderer 'netplan'" in log
        client.execute(
            "mv /var/log/cloud-init.log /var/log/cloud-init.log.bak"
        )
        netplan = yaml.safe_load(
            client.execute("cat /etc/netplan/50-cloud-init.yaml")
        )

        client.restart()

        log = client.read_from_file("/var/log/cloud-init.log")
        assert "Event Allowed: scope=network EventType=boot" in log
        assert "Applying network configuration" in log
        assert "Running command ['netplan', 'generate']" not in log
        assert (
            "skipping call to `netplan generate`."
            " reason: identical netplan config"
        ) in log
        netplan_new = yaml.safe_load(
            client.execute("cat /etc/netplan/50-cloud-init.yaml")
        )
        assert netplan == netplan_new, "no changes expected in netplan config"

    def test_applied(self, client: IntegrationInstance):
        log = client.read_from_file("/var/log/cloud-init.log")
        assert "Applying network configuration" in log
        assert "Selected renderer 'netplan'" in log
        client.execute(
            "mv /var/log/cloud-init.log /var/log/cloud-init.log.bak"
        )
        # fake a change in the rendered network config file
        _add_dummy_bridge_to_netplan(client)
        netplan = yaml.safe_load(
            client.execute("cat /etc/netplan/50-cloud-init.yaml")
        )

        client.restart()

        log = client.read_from_file("/var/log/cloud-init.log")
        assert "Event Allowed: scope=network EventType=boot" in log
        assert "Applying network configuration" in log
        assert (
            "skipping call to `netplan generate`."
            " reason: identical netplan config"
        ) not in log
        assert "Running command ['netplan', 'generate']" in log
        netplan_new = yaml.safe_load(
            client.execute("cat /etc/netplan/50-cloud-init.yaml")
        )
        assert netplan != netplan_new, "changes expected in netplan config"


@pytest.mark.skipif(PLATFORM != "ec2", reason="test is ec2 specific")
def test_ec2_multi_ip(setup_image, session_cloud: IntegrationCloud):
    ec2 = session_cloud.cloud_instance.client
    with session_cloud.launch(launch_kwargs={}, user_data=None) as client:
        nic_id = client.instance._instance.network_interfaces[0].id
        res = ec2.assign_private_ip_addresses(
            NetworkInterfaceId=nic_id, SecondaryPrivateIpAddressCount=1
        )
        assert res["ResponseMetadata"]["HTTPStatusCode"] == 200
        secondary_priv_ip = res["AssignedPrivateIpAddresses"][0][
            "PrivateIpAddress"
        ]
        instance_pub_ip = client.instance.ip

        # Create Elastic IP
        allocation = ec2.allocate_address(Domain="vpc")
        try:
            secondary_pub_ip = allocation["PublicIp"]

            res = ec2.associate_address(
                AllocationId=allocation["AllocationId"],
                NetworkInterfaceId=nic_id,
                PrivateIpAddress=secondary_priv_ip,
            )
            assert res["ResponseMetadata"]["HTTPStatusCode"] == 200
            client.execute("cloud-init clean --logs")
            client.restart()

            # SSH over primary NIC works
            subp("nc -w 1 -zv " + instance_pub_ip + " 22", shell=True)

            import pdb

            pdb.set_trace()
            # THE TEST: SSH over secondary NIC works
            subp("nc -w 1 -zv " + secondary_pub_ip + " 22", shell=True)
        finally:
            with contextlib.suppress(Exception):
                ec2.release_address(AllocationId=allocation["AllocationId"])
