// SPDX-License-Identifier: Apache-2.0

use std::collections::HashMap;
use std::convert::TryFrom;
use std::marker::PhantomData;
use std::str::FromStr;

use serde::{de, de::Visitor, Deserialize, Deserializer, Serialize};

use crate::{
    BaseInterface, BridgePortVlanConfig, ErrorKind, InterfaceType,
    NmstateError, VlanProtocol,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct LinuxBridgeInterface {
    #[serde(flatten)]
    pub base: BaseInterface,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bridge: Option<LinuxBridgeConfig>,
}

impl Default for LinuxBridgeInterface {
    fn default() -> Self {
        let mut base = BaseInterface::new();
        base.iface_type = InterfaceType::LinuxBridge;
        Self { base, bridge: None }
    }
}

impl LinuxBridgeInterface {
    // Return None when desire state does not mentioned ports.
    pub(crate) fn ports(&self) -> Option<Vec<&str>> {
        self.bridge
            .as_ref()
            .and_then(|br_conf| br_conf.port.as_ref())
            .map(|ports| {
                ports.as_slice().iter().map(|p| p.name.as_str()).collect()
            })
    }

    pub fn new() -> Self {
        Self::default()
    }

    pub(crate) fn pre_edit_cleanup(&self) -> Result<(), NmstateError> {
        self.bridge
            .as_ref()
            .map(LinuxBridgeConfig::pre_edit_cleanup)
            .transpose()?;
        Ok(())
    }

    pub(crate) fn get_port_conf(
        &self,
        port_name: &str,
    ) -> Option<&LinuxBridgePortConfig> {
        self.bridge
            .as_ref()
            .and_then(|br_conf| br_conf.port.as_ref())
            .and_then(|port_confs| {
                port_confs
                    .iter()
                    .find(|port_conf| port_conf.name == port_name)
            })
    }

    pub(crate) fn vlan_filtering_is_enabled(&self) -> bool {
        self.bridge
            .as_ref()
            .map_or(false, LinuxBridgeConfig::vlan_filtering_is_enabled)
    }

    // Port name list change is not this function's responsibility, top level
    // code will take care of it.
    // This function only find out those port which has changed vlan or stp
    // configuration.
    pub(crate) fn get_config_changed_ports(&self, current: &Self) -> Vec<&str> {
        let mut ret: Vec<&str> = Vec::new();
        let mut des_ports_index: HashMap<&str, &LinuxBridgePortConfig> =
            HashMap::new();
        let mut cur_ports_index: HashMap<&str, &LinuxBridgePortConfig> =
            HashMap::new();
        if let Some(port_confs) = self
            .bridge
            .as_ref()
            .and_then(|br_conf| br_conf.port.as_ref())
        {
            for port_conf in port_confs {
                des_ports_index.insert(port_conf.name.as_str(), port_conf);
            }
        }

        if let Some(port_confs) = current
            .bridge
            .as_ref()
            .and_then(|br_conf| br_conf.port.as_ref())
        {
            for port_conf in port_confs {
                cur_ports_index.insert(port_conf.name.as_str(), port_conf);
            }
        }

        for (port_name, port_conf) in des_ports_index.iter() {
            if let Some(cur_port_conf) = cur_ports_index.get(port_name) {
                if port_conf.is_changed(cur_port_conf) {
                    ret.push(port_name);
                }
            }
        }
        ret
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
#[non_exhaustive]
pub struct LinuxBridgeConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub options: Option<LinuxBridgeOptions>,
    #[serde(skip_serializing_if = "Option::is_none", alias = "ports")]
    pub port: Option<Vec<LinuxBridgePortConfig>>,
}

impl LinuxBridgeConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub(crate) fn pre_edit_cleanup(&self) -> Result<(), NmstateError> {
        self.options
            .as_ref()
            .map(LinuxBridgeOptions::pre_edit_cleanup)
            .transpose()?;
        Ok(())
    }

    pub(crate) fn vlan_filtering_is_enabled(&self) -> bool {
        self.port.as_ref().map_or(false, |p| {
            p.iter()
                .any(LinuxBridgePortConfig::vlan_filtering_is_enabled)
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
#[non_exhaustive]
pub struct LinuxBridgePortConfig {
    pub name: String,
    #[serde(
        skip_serializing_if = "Option::is_none",
        default,
        deserialize_with = "crate::deserializer::option_bool_or_string"
    )]
    pub stp_hairpin_mode: Option<bool>,
    #[serde(
        skip_serializing_if = "Option::is_none",
        default,
        deserialize_with = "crate::deserializer::option_u32_or_string"
    )]
    pub stp_path_cost: Option<u32>,
    #[serde(
        skip_serializing_if = "Option::is_none",
        default,
        deserialize_with = "crate::deserializer::option_u16_or_string"
    )]
    pub stp_priority: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vlan: Option<BridgePortVlanConfig>,
}

impl LinuxBridgePortConfig {
    pub fn new() -> Self {
        Self::default()
    }

    fn is_changed(&self, current: &Self) -> bool {
        (self.stp_hairpin_mode.is_some()
            && self.stp_hairpin_mode != current.stp_hairpin_mode)
            || (self.stp_path_cost.is_some()
                && self.stp_path_cost != current.stp_path_cost)
            || (self.stp_priority.is_some()
                && self.stp_priority != current.stp_priority)
            || match (self.vlan.as_ref(), current.vlan.as_ref()) {
                (Some(des_vlan_conf), Some(cur_vlan_conf)) => {
                    (des_vlan_conf.is_empty() && !cur_vlan_conf.is_empty())
                        || des_vlan_conf.is_changed(cur_vlan_conf)
                }
                (Some(_), None) => true,
                _ => false,
            }
    }

    fn vlan_filtering_is_enabled(&self) -> bool {
        self.vlan
            .as_ref()
            .map_or(true, |v| *v != BridgePortVlanConfig::default())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
#[non_exhaustive]
pub struct LinuxBridgeOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gc_timer: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub group_addr: Option<String>,
    // The group_forward_mask is the same with group_fwd_mask. The former is
    // used by NetworkManager, the later is used by sysfs. Nmstate support
    // both.
    #[serde(
        skip_serializing_if = "Option::is_none",
        default,
        deserialize_with = "crate::deserializer::option_u16_or_string"
    )]
    pub group_forward_mask: Option<u16>,
    #[serde(
        skip_serializing_if = "Option::is_none",
        default,
        deserialize_with = "crate::deserializer::option_u16_or_string"
    )]
    pub group_fwd_mask: Option<u16>,
    #[serde(
        skip_serializing_if = "Option::is_none",
        default,
        deserialize_with = "crate::deserializer::option_u32_or_string"
    )]
    pub hash_max: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hello_timer: Option<u64>,
    #[serde(
        skip_serializing_if = "Option::is_none",
        default,
        deserialize_with = "crate::deserializer::option_u32_or_string"
    )]
    pub mac_ageing_time: Option<u32>,
    #[serde(
        skip_serializing_if = "Option::is_none",
        default,
        deserialize_with = "crate::deserializer::option_u32_or_string"
    )]
    pub multicast_last_member_count: Option<u32>,
    #[serde(
        skip_serializing_if = "Option::is_none",
        default,
        deserialize_with = "crate::deserializer::option_u64_or_string"
    )]
    pub multicast_last_member_interval: Option<u64>,
    #[serde(
        skip_serializing_if = "Option::is_none",
        default,
        deserialize_with = "crate::deserializer::option_u64_or_string"
    )]
    pub multicast_membership_interval: Option<u64>,
    #[serde(
        skip_serializing_if = "Option::is_none",
        default,
        deserialize_with = "crate::deserializer::option_bool_or_string"
    )]
    pub multicast_querier: Option<bool>,
    #[serde(
        skip_serializing_if = "Option::is_none",
        default,
        deserialize_with = "crate::deserializer::option_u64_or_string"
    )]
    pub multicast_querier_interval: Option<u64>,
    #[serde(
        skip_serializing_if = "Option::is_none",
        default,
        deserialize_with = "crate::deserializer::option_u64_or_string"
    )]
    pub multicast_query_interval: Option<u64>,
    #[serde(
        skip_serializing_if = "Option::is_none",
        default,
        deserialize_with = "crate::deserializer::option_u64_or_string"
    )]
    pub multicast_query_response_interval: Option<u64>,
    #[serde(
        skip_serializing_if = "Option::is_none",
        default,
        deserialize_with = "crate::deserializer::option_bool_or_string"
    )]
    pub multicast_query_use_ifaddr: Option<bool>,
    #[serde(
        skip_serializing_if = "Option::is_none",
        default,
        deserialize_with = "de_multicast_router"
    )]
    pub multicast_router: Option<LinuxBridgeMulticastRouterType>,
    #[serde(
        skip_serializing_if = "Option::is_none",
        default,
        deserialize_with = "crate::deserializer::option_bool_or_string"
    )]
    pub multicast_snooping: Option<bool>,
    #[serde(
        skip_serializing_if = "Option::is_none",
        default,
        deserialize_with = "crate::deserializer::option_u32_or_string"
    )]
    pub multicast_startup_query_count: Option<u32>,
    #[serde(
        skip_serializing_if = "Option::is_none",
        default,
        deserialize_with = "crate::deserializer::option_u64_or_string"
    )]
    pub multicast_startup_query_interval: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stp: Option<LinuxBridgeStpOptions>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vlan_protocol: Option<VlanProtocol>,
}

impl LinuxBridgeOptions {
    pub fn new() -> Self {
        Self::default()
    }

    pub(crate) fn pre_edit_cleanup(&self) -> Result<(), NmstateError> {
        self.stp
            .as_ref()
            .map(LinuxBridgeStpOptions::pre_edit_cleanup)
            .transpose()?;
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case", deny_unknown_fields)]
#[non_exhaustive]
pub struct LinuxBridgeStpOptions {
    #[serde(
        skip_serializing_if = "Option::is_none",
        default,
        deserialize_with = "crate::deserializer::option_bool_or_string"
    )]
    pub enabled: Option<bool>,
    #[serde(
        skip_serializing_if = "Option::is_none",
        default,
        deserialize_with = "crate::deserializer::option_u8_or_string"
    )]
    pub forward_delay: Option<u8>,
    #[serde(
        skip_serializing_if = "Option::is_none",
        default,
        deserialize_with = "crate::deserializer::option_u8_or_string"
    )]
    pub hello_time: Option<u8>,
    #[serde(
        skip_serializing_if = "Option::is_none",
        default,
        deserialize_with = "crate::deserializer::option_u8_or_string"
    )]
    pub max_age: Option<u8>,
    #[serde(
        skip_serializing_if = "Option::is_none",
        default,
        deserialize_with = "crate::deserializer::option_u16_or_string"
    )]
    pub priority: Option<u16>,
}

impl LinuxBridgeStpOptions {
    pub const HELLO_TIME_MAX: u8 = 10;
    pub const HELLO_TIME_MIN: u8 = 1;
    pub const MAX_AGE_MAX: u8 = 40;
    pub const MAX_AGE_MIN: u8 = 6;
    pub const FORWARD_DELAY_MAX: u8 = 30;
    pub const FORWARD_DELAY_MIN: u8 = 2;

    pub fn new() -> Self {
        Self::default()
    }

    pub(crate) fn pre_edit_cleanup(&self) -> Result<(), NmstateError> {
        if let Some(hello_time) = self.hello_time {
            if !(Self::HELLO_TIME_MIN..=Self::HELLO_TIME_MAX)
                .contains(&hello_time)
            {
                let e = NmstateError::new(
                    ErrorKind::InvalidArgument,
                    format!(
                        "Desired STP hello time {} is not in the range \
                        of [{},{}]",
                        hello_time,
                        Self::HELLO_TIME_MIN,
                        Self::HELLO_TIME_MAX
                    ),
                );
                log::error!("{}", e);
                return Err(e);
            }
        }

        if let Some(max_age) = self.max_age {
            if !(Self::MAX_AGE_MIN..=Self::MAX_AGE_MAX).contains(&max_age) {
                let e = NmstateError::new(
                    ErrorKind::InvalidArgument,
                    format!(
                        "Desired STP max age {} is not in the range \
                        of [{},{}]",
                        max_age,
                        Self::MAX_AGE_MIN,
                        Self::MAX_AGE_MAX
                    ),
                );
                log::error!("{}", e);
                return Err(e);
            }
        }
        if let Some(forward_delay) = self.forward_delay {
            if !(Self::FORWARD_DELAY_MIN..=Self::FORWARD_DELAY_MAX)
                .contains(&forward_delay)
            {
                let e = NmstateError::new(
                    ErrorKind::InvalidArgument,
                    format!(
                        "Desired STP forward delay {} is not in the range \
                        of [{},{}]",
                        forward_delay,
                        Self::FORWARD_DELAY_MIN,
                        Self::FORWARD_DELAY_MAX
                    ),
                );
                log::error!("{}", e);
                return Err(e);
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub enum LinuxBridgeMulticastRouterType {
    Auto = 1,
    Disabled = 0,
    Enabled = 2,
}

impl Default for LinuxBridgeMulticastRouterType {
    fn default() -> Self {
        Self::Auto
    }
}

impl FromStr for LinuxBridgeMulticastRouterType {
    type Err = NmstateError;
    fn from_str(s: &str) -> Result<Self, NmstateError> {
        match s.to_lowercase().as_str() {
            "auto" => Ok(Self::Auto),
            "disabled" => Ok(Self::Disabled),
            "enabled" => Ok(Self::Enabled),
            _ => Err(NmstateError::new(
                ErrorKind::InvalidArgument,
                format!(
                    "Invalid linux bridge multicast_router type {}, \
                    expecting 0|1|2 or auto|disabled|enabled",
                    s
                ),
            )),
        }
    }
}

impl TryFrom<u64> for LinuxBridgeMulticastRouterType {
    type Error = NmstateError;
    fn try_from(value: u64) -> Result<Self, NmstateError> {
        match value {
            x if x == Self::Auto as u64 => Ok(Self::Auto),
            x if x == Self::Disabled as u64 => Ok(Self::Disabled),
            x if x == Self::Enabled as u64 => Ok(Self::Enabled),
            _ => Err(NmstateError::new(
                ErrorKind::InvalidArgument,
                format!(
                    "Invalid linux bridge multicast_router type {}, \
                    expecting 0|1|2 or auto|disabled|enabled",
                    value
                ),
            )),
        }
    }
}

impl std::fmt::Display for LinuxBridgeMulticastRouterType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Auto => "auto",
                Self::Disabled => "disabled",
                Self::Enabled => "enabled",
            }
        )
    }
}

pub(crate) fn de_multicast_router<'de, D>(
    deserializer: D,
) -> Result<Option<LinuxBridgeMulticastRouterType>, D::Error>
where
    D: Deserializer<'de>,
{
    struct IntegerOrString(
        PhantomData<fn() -> Option<LinuxBridgeMulticastRouterType>>,
    );

    impl<'de> Visitor<'de> for IntegerOrString {
        type Value = Option<LinuxBridgeMulticastRouterType>;

        fn expecting(
            &self,
            formatter: &mut std::fmt::Formatter,
        ) -> std::fmt::Result {
            formatter.write_str("integer 0|1|2 or string auto|disabled|enabled")
        }

        fn visit_str<E>(
            self,
            value: &str,
        ) -> Result<Option<LinuxBridgeMulticastRouterType>, E>
        where
            E: de::Error,
        {
            FromStr::from_str(value)
                .map_err(de::Error::custom)
                .map(Some)
        }

        fn visit_u64<E>(
            self,
            value: u64,
        ) -> Result<Option<LinuxBridgeMulticastRouterType>, E>
        where
            E: de::Error,
        {
            LinuxBridgeMulticastRouterType::try_from(value)
                .map_err(de::Error::custom)
                .map(Some)
        }
    }

    deserializer.deserialize_any(IntegerOrString(PhantomData))
}
