pub const UNAVAILABLE: &str = "unavailable";
pub const DI_ENABLED: &str = "enabled";
pub const DI_DISABLED: &str = "disabled";

pub const PATH_RUN: &str = "run";
pub const PATH_SYS_CLASS_DMI_ID: &str = "sys/class/dmi/id";
pub const PATH_SYS_HYPERVISOR: &str = "sys/hypervisor";
pub const PATH_SYS_CLASS_BLOCK: &str = "sys/class/block";
pub const PATH_DEV_DISK: &str = "dev/disk";
pub const PATH_VAR_LIB_CLOUD: &str = "var/lib/cloud";
pub const PATH_DI_CONFIG: &str = "etc/cloud/ds-identify.cfg";
pub const PATH_PROC_CMDLINE: &str = "proc/cmdline";
pub const PATH_PROC_1_CMDLINE: &str = "proc/1/cmdline";
pub const PATH_PROC_1_ENVIRON: &str = "proc/1/environ";
pub const PATH_PROC_UPTIME: &str = "proc/uptime";
pub const PATH_ETC_CLOUD: &str = "etc/cloud";
pub const PATH_ETC_CI_CFG: &str = "cloud.cfg";
pub const PATH_RUN_CI: &str = "cloud-init";
pub const PATH_RUN_CI_CFG: &str = "cloud.cfg";
pub const PATH_RUN_DI_RESULT: &str = ".ds-identify.result";

pub const DI_DSLIST_DEFAULT: &str = "MAAS ConfigDrive NoCloud AltCloud Azure Bigstep \
CloudSigma CloudStack DigitalOcean Vultr AliYun Ec2 GCE OpenNebula OpenStack \
OVF SmartOS Scaleway Hetzner IBMCloud Oracle Exoscale RbxCloud UpCloud VMware \
LXD NWCS";
