// SPDX-License-Identifier: Apache-2.0

mod bond;
mod bridge;
mod connection;
mod dns;
mod ethtool;
mod ieee8021x;
mod infiniband;
mod inter_connections;
mod ip;
mod mac_vlan;
mod mptcp;
mod ovs;
mod route;
mod route_rule;
mod sriov;
mod user;
mod veth;
mod vlan;
mod vrf;
mod vxlan;
mod wired;

pub(crate) use self::connection::iface_to_nm_connections;
#[cfg(feature = "query_apply")]
pub(crate) use self::connection::{
    get_exist_profile, iface_type_to_nm, NM_SETTING_BOND_SETTING_NAME,
    NM_SETTING_BRIDGE_SETTING_NAME, NM_SETTING_DUMMY_SETTING_NAME,
    NM_SETTING_INFINIBAND_SETTING_NAME, NM_SETTING_MACVLAN_SETTING_NAME,
    NM_SETTING_OVS_BRIDGE_SETTING_NAME, NM_SETTING_OVS_IFACE_SETTING_NAME,
    NM_SETTING_OVS_PORT_SETTING_NAME, NM_SETTING_VETH_SETTING_NAME,
    NM_SETTING_VLAN_SETTING_NAME, NM_SETTING_VRF_SETTING_NAME,
    NM_SETTING_VXLAN_SETTING_NAME, NM_SETTING_WIRED_SETTING_NAME,
};
pub(crate) use self::inter_connections::{
    use_uuid_for_controller_reference, use_uuid_for_parent_reference,
};

#[cfg(feature = "query_apply")]
pub(crate) use self::user::NMSTATE_DESCRIPTION;
