// SPDX-License-Identifier: Apache-2.0

use std::collections::HashMap;
use std::iter::FromIterator;

use super::super::nm_dbus::{
    NmConnection, NmSettingOvsDpdk, NmSettingOvsExtIds, NmSettingOvsIface,
    NmSettingOvsPatch,
};

use super::connection::gen_nm_conn_setting;

use crate::{
    BaseInterface, Interface, InterfaceType, NmstateError, OvsBridgeBondMode,
    OvsBridgeInterface, OvsBridgePortConfig, OvsInterface, UnknownInterface,
};

pub(crate) fn create_ovs_port_nm_conn(
    br_name: &str,
    port_conf: &OvsBridgePortConfig,
    exist_nm_conn: Option<&NmConnection>,
    stable_uuid: bool,
) -> Result<NmConnection, NmstateError> {
    let mut nm_conn = exist_nm_conn.cloned().unwrap_or_default();
    let mut base_iface = BaseInterface::new();
    base_iface.name = port_conf.name.clone();
    base_iface.iface_type = InterfaceType::Other("ovs-port".to_string());
    base_iface.controller = Some(br_name.to_string());
    base_iface.controller_type = Some(InterfaceType::OvsBridge);
    let mut iface = UnknownInterface::new();
    iface.base = base_iface;
    gen_nm_conn_setting(&Interface::Unknown(iface), &mut nm_conn, stable_uuid)?;

    let mut nm_ovs_port_set =
        nm_conn.ovs_port.as_ref().cloned().unwrap_or_default();
    if let Some(bond_conf) = &port_conf.bond {
        if let Some(bond_mode) = &bond_conf.mode {
            match bond_mode {
                OvsBridgeBondMode::Lacp => {
                    nm_ovs_port_set.lacp = Some("active".into());
                }
                OvsBridgeBondMode::ActiveBackup
                | OvsBridgeBondMode::BalanceSlb => {
                    nm_ovs_port_set.lacp = Some("off".into());
                    nm_ovs_port_set.mode = Some(bond_mode.to_string());
                }
                OvsBridgeBondMode::BalanceTcp => {
                    nm_ovs_port_set.lacp = Some("active".into());
                    nm_ovs_port_set.mode = Some(bond_mode.to_string());
                }
            };
        }

        if let Some(bond_downdelay) = bond_conf.bond_downdelay {
            nm_ovs_port_set.down_delay = Some(bond_downdelay);
        }

        if let Some(bond_updelay) = bond_conf.bond_updelay {
            nm_ovs_port_set.up_delay = Some(bond_updelay);
        }
    }
    if let Some(vlan_conf) = port_conf.vlan.as_ref() {
        if let Some(tag) = vlan_conf.tag {
            nm_ovs_port_set.tag = Some(tag.into());
        }
        if let Some(vlan_mode) = vlan_conf.mode {
            nm_ovs_port_set.vlan_mode = Some(vlan_mode.to_string());
        }
    }
    nm_conn.ovs_port = Some(nm_ovs_port_set);
    Ok(nm_conn)
}

pub(crate) fn get_ovs_port_name(
    ovs_br_iface: &OvsBridgeInterface,
    ovs_iface_name: &str,
    cur_ovs_br_iface: Option<&Interface>,
) -> Option<String> {
    let port_confs = if ovs_br_iface.ports().is_none() {
        if let Some(Interface::OvsBridge(cur_ovs_br_iface)) = cur_ovs_br_iface {
            cur_ovs_br_iface.port_confs()
        } else {
            ovs_br_iface.port_confs()
        }
    } else {
        ovs_br_iface.port_confs()
    };
    for port_conf in port_confs {
        if let Some(bond_conf) = &port_conf.bond {
            for bond_port_name in bond_conf.ports() {
                if bond_port_name == ovs_iface_name {
                    return Some(port_conf.name.as_str().to_string());
                }
            }
        } else if ovs_iface_name == port_conf.name {
            return Some(ovs_iface_name.to_string());
        }
    }
    None
}

pub(crate) fn gen_nm_ovs_br_setting(
    ovs_br_iface: &OvsBridgeInterface,
    nm_conn: &mut NmConnection,
) {
    let mut nm_ovs_br_set =
        nm_conn.ovs_bridge.as_ref().cloned().unwrap_or_default();

    if let Some(br_conf) = &ovs_br_iface.bridge {
        if let Some(br_opts) = &br_conf.options {
            nm_ovs_br_set.stp = br_opts.stp;
            nm_ovs_br_set.rstp = br_opts.rstp;
            nm_ovs_br_set.mcast_snooping_enable = br_opts.mcast_snooping_enable;
            if let Some(fail_mode) = &br_opts.fail_mode {
                if !fail_mode.is_empty() {
                    nm_ovs_br_set.fail_mode = Some(fail_mode.to_string());
                }
            }
            if let Some(dp_type) = &br_opts.datapath {
                if !dp_type.is_empty() {
                    nm_ovs_br_set.datapath_type = Some(dp_type.to_string());
                }
            }
        }
    }
    nm_conn.ovs_bridge = Some(nm_ovs_br_set);
}

pub(crate) fn gen_nm_ovs_iface_setting(
    iface: &OvsInterface,
    nm_conn: &mut NmConnection,
) {
    if let Some(peer) = iface
        .patch
        .as_ref()
        .map(|patch_conf| patch_conf.peer.as_str())
    {
        let mut nm_ovs_iface_set =
            nm_conn.ovs_iface.as_ref().cloned().unwrap_or_default();
        nm_ovs_iface_set.iface_type = Some("patch".to_string());
        let mut nm_ovs_patch = NmSettingOvsPatch::default();
        nm_ovs_patch.peer = Some(peer.to_string());
        nm_conn.ovs_patch = Some(nm_ovs_patch);
        nm_conn.ovs_iface = Some(nm_ovs_iface_set);
    } else if let Some(dpdk_iface) = iface.dpdk.as_ref() {
        if !dpdk_iface.devargs.is_empty() {
            let mut nm_ovs_iface_set =
                nm_conn.ovs_iface.as_ref().cloned().unwrap_or_default();
            nm_ovs_iface_set.iface_type = Some("dpdk".to_string());
            let mut nm_ovs_dpdk = NmSettingOvsDpdk::default();
            nm_ovs_dpdk.devargs = Some(dpdk_iface.devargs.to_string());
            nm_ovs_dpdk.n_rxq = dpdk_iface.rx_queue;
            nm_conn.ovs_dpdk = Some(nm_ovs_dpdk);
            nm_conn.ovs_iface = Some(nm_ovs_iface_set);
        }
    }
    if nm_conn.ovs_iface.is_none() {
        let mut nm_set = NmSettingOvsIface::default();
        nm_set.iface_type = Some("internal".to_string());
        nm_conn.ovs_iface = Some(nm_set);
    }
}

pub(crate) fn gen_nm_ovs_ext_ids_setting(
    iface: &Interface,
    nm_conn: &mut NmConnection,
) {
    if iface.iface_type() != InterfaceType::OvsBridge
        && iface.base_iface().controller_type != Some(InterfaceType::OvsBridge)
    {
        nm_conn.ovs_ext_ids = None;
    } else if let Some(conf) = iface.base_iface().ovsdb.as_ref() {
        let mut nm_setting = NmSettingOvsExtIds::default();
        nm_setting.data = Some(HashMap::from_iter(
            conf.get_external_ids()
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string())),
        ));
        nm_conn.ovs_ext_ids = Some(nm_setting);
    }
}
