//!
//! # Param Structure for Caller
//!

use crate::{
    ctrl::{ENGINE, TEMPLATE},
    e,
    err::*,
    model::{
        Hardware, NetAddr, NetKind, Vm, VmFeature, VmResource, VmState, DEFAULT_ID,
    },
};
use ruc::*;
use serde::{Deserialize, Serialize};
use std::{collections::HashSet, sync::Arc};

/// Infomations about a VM instance.
#[derive(Default, Deserialize, Serialize)]
pub struct VmCfg {
    /// Name of the `VM`.
    pub name: Option<String>,
    /// Engine name wanted.
    pub engine: Option<String>,
    /// Template name wanted.
    pub template: String,
    /// Network kind of this VM.
    pub net_kind: NetKind,
    /// Hardware resource of VM.
    pub hw: Hardware,
    /// Usually an 'IP' or a 'domain url'.
    ///
    /// Only meaningful from the perspective of the client,
    /// to indicate how to connect to it from the client.
    ///
    /// This has different meanings with the
    /// [ip_addr](crate::model::VmResource::ip_addr) in [VmResource](crate::model::VmResource).
    pub addr: NetAddr,
    /// Features required by this vm.
    pub features: HashSet<VmFeature>,
}

impl From<&Vm> for VmCfg {
    fn from(vm: &Vm) -> Self {
        VmCfg {
            name: vm.name.clone(),
            engine: Some(vm.engine.name().to_owned()),
            template: vm.template.name.clone(),
            net_kind: vm.net_kind.clone(),
            hw: vm.resource.hw.clone(),
            addr: vm.addr.clone(),
            features: vm.features.clone(),
        }
    }
}

impl VmCfg {
    /// Convert a [VmCfg](self::VmCfg) to a [Vm](crate::model::Vm).
    pub fn into_vm(cfg: VmCfg) -> Result<Vm> {
        let template = TEMPLATE
            .get(&cfg.template)
            .ok_or(e!(ERR_TT_CTRL_TEMPLATE_NOT_FOUND))?;
        let engine = cfg
            .engine
            .as_ref()
            .or_else(|| template.compatible_engines.iter().next())
            .and_then(|e| ENGINE.get(e))
            .map(Arc::clone)
            .ok_or(e!(ERR_TT_CTRL_ENGINE_NOT_FOUND))?;
        Ok(Vm {
            id: DEFAULT_ID,
            name: cfg.name,
            engine,
            template,
            runtime_image: String::new(),
            net_kind: cfg.net_kind,
            snapshots: map! {},
            latest_meta: None,
            state: VmState::default(),
            resource: VmResource {
                hw: cfg.hw,
                ..VmResource::default()
            },
            addr: cfg.addr,
            features: cfg.features,
        })
    }
}
