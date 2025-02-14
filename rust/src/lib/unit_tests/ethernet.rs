use crate::{ErrorKind, EthernetInterface, Interfaces};

#[test]
fn test_ethernet_stringlized_attributes() {
    let iface: EthernetInterface = serde_yaml::from_str(
        r#"---
name: eth1
type: ethernet
state: up
ethernet:
  auto-negotiation: "false"
  speed: "1000"
  sr-iov:
    total-vfs: "64"
    vfs:
      - id: "0"
        spoof-check: "true"
        trust: "false"
        min-tx-rate: "100"
        max-tx-rate: "101"
        vlan-id: "102"
        qos: "103"
"#,
    )
    .unwrap();

    let eth_conf = iface.ethernet.unwrap();
    let sriov_conf = eth_conf.sr_iov.as_ref().unwrap();
    let vf_conf = sriov_conf.vfs.as_ref().unwrap().get(0).unwrap();

    assert_eq!(eth_conf.speed, Some(1000));
    assert_eq!(sriov_conf.total_vfs, Some(64));
    assert_eq!(vf_conf.id, 0);
    assert_eq!(vf_conf.spoof_check, Some(true));
    assert_eq!(vf_conf.trust, Some(false));
    assert_eq!(vf_conf.min_tx_rate, Some(100));
    assert_eq!(vf_conf.max_tx_rate, Some(101));
    assert_eq!(vf_conf.vlan_id, Some(102));
    assert_eq!(vf_conf.qos, Some(103));
}

#[test]
fn test_veth_change_peer_away_from_ignored_peer() {
    let desired: Interfaces = serde_yaml::from_str(
        r#"---
- name: veth1
  type: veth
  state: up
  veth:
    peer: newpeer
"#,
    )
    .unwrap();
    let current: Interfaces = serde_yaml::from_str(
        r#"---
- name: veth1
  type: veth
  state: up
  veth:
    peer: veth1peer

- name: veth1peer
  type: veth
  state: ignore
  veth:
    peer: veth1
"#,
    )
    .unwrap();

    let ignored_kernel_ifaces = vec!["veth1peer".to_string()];

    let result = desired.pre_ignore_check(&current, &ignored_kernel_ifaces);
    assert!(result.is_err());
    if let Err(e) = result {
        assert_eq!(e.kind(), ErrorKind::InvalidArgument);
    }
}

#[test]
fn test_eth_verify_absent_ignore_current_up() {
    let desired: Interfaces = serde_yaml::from_str(
        r#"---
- name: eth1
  type: ethernet
  state: absent
"#,
    )
    .unwrap();
    let current: Interfaces = serde_yaml::from_str(
        r#"---
- name: eth1
  type: ethernet
  state: up
"#,
    )
    .unwrap();

    desired.verify(&Interfaces::new(), &current).unwrap();
}
