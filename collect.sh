#!/bin/bash

set -eux
outdir=./tmp/output

mkdir -p "${outdir}/jammy"
mkdir -p "${outdir}/focal"
mkdir -p "${outdir}/bionic"

export CLOUD_INIT_PLATFORM="gce"
# export CLOUD_INIT_PLATFORM="lxd_container"
export CLOUD_INIT_COLLECT_LOGS="NEVER"
export CLOUD_INIT_LOCAL_LOG_PATH="/tmp/cloud_init_test_logs"
export CLOUD_INIT_KEEP_INSTANCE=False

for os_image in jammy focal bionic
do
	export CLOUD_OS_IMAGE=${os_image}
	for ((i = 0; i < 8; i++)); do
		timestamp=$(date +%s)
		tox -e integration-tests -- \
			tests/integration_tests/datasources/test_gce_nic_boot_time.py::test_network_update_on_reboot

		cp /tmp/1st-boot-analysis.tar.xz \
			"${outdir}/${CLOUD_OS_IMAGE}/${timestamp}-update-1s-boot-analysis.tar.xz"
		cp /tmp/2nd-boot-analysis.tar.xz \
			"${outdir}/${CLOUD_OS_IMAGE}/${timestamp}-update-2nd-boot-analysis.tar.xz"

		tox -e integration-tests -- \
			tests/integration_tests/datasources/test_gce_nic_boot_time.py::test_network_baseline
		cp /tmp/1st-boot-analysis.tar.xz \
			"${outdir}/${CLOUD_OS_IMAGE}/${timestamp}-baseline-1s-boot-analysis.tar.xz"
		cp /tmp/2nd-boot-analysis.tar.xz \
			"${outdir}/${CLOUD_OS_IMAGE}/${timestamp}-baseline-2nd-boot-analysis.tar.xz"
	done
done
