//!
//! # Resource Controller
//!
//! All resoures are under this moduler's management.
//!

use crate::{
    err::*,
    model::{Env, EnvId, Hardware, User, UserId, Vm, VmEngine, VmId, VmTemplate},
};
use once_cell::sync::Lazy;
use parking_lot::RwLock;
use ruc::*;
use std::{
    collections::{HashMap, HashSet},
    ops::Deref,
    sync::Arc,
};

/// Global entrypoint.
pub static SERV: Lazy<ServCtrl> = Lazy::new(|| Arc::new(Service::default()));
/// Collections of vm-engines, can NOT be changed in runtime.
pub static ENGINE: Lazy<EngineCtrl> = Lazy::new(|| pnk!(EngineCtrl::init(None)));
/// Collections of vm-templates, can be updated in runtime.
pub static TEMPLATE: Lazy<TemplateCtrl> = Lazy::new(TemplateCtrl::default);
/// Harewares of the host, can NOT be changed in runtime.
pub static HARDWARE: Lazy<HardwareCtrl> = Lazy::new(|| pnk!(HardwareCtrl::init(None)));

type ServCtrl = Arc<Service>;

/// Service is a global data collection.
#[derive(Default)]
pub struct Service {
    #[allow(missing_docs)]
    pub all_user: Arc<RwLock<HashMap<UserId, User>>>,
    #[allow(missing_docs)]
    pub all_env: Arc<RwLock<HashMap<EnvId, Env>>>,
    #[allow(missing_docs)]
    pub all_vm: Arc<RwLock<HashMap<VmId, Vm>>>,
}

/// {Vm Engine Name} => {Vm Engine Object}
#[derive(Clone)]
pub struct EngineCtrl(Arc<HashMap<String, Arc<dyn VmEngine>>>);

impl EngineCtrl {
    /// Caller(user) uses this function to init [ENGINES](self::ENGINES).
    pub fn init(em: Option<Vec<Arc<dyn VmEngine>>>) -> Option<EngineCtrl> {
        static mut EM: Option<EngineCtrl> = None;

        unsafe {
            if let Some(e) = EM.as_ref() {
                Some(e.clone())
            } else if let Some(e) = em {
                let ret = EngineCtrl(Arc::new(
                    e.into_iter().map(|ve| (ve.name().to_owned(), ve)).collect(),
                ));
                EM = Some(ret.clone());
                Some(ret)
            } else {
                None
            }
        }
    }
}

impl Deref for EngineCtrl {
    type Target = Arc<HashMap<String, Arc<dyn VmEngine>>>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// The container of vm templates,
/// {Vm Template Name} => {Vm Template Object}
#[derive(Default)]
pub struct TemplateCtrl(Arc<RwLock<HashMap<String, Arc<VmTemplate>>>>);

impl TemplateCtrl {
    /// Replace the whole data with a new one.
    #[inline(always)]
    pub fn reinit(&mut self, t: HashMap<String, VmTemplate>) {
        *self.write() = t.into_iter().map(|(k, v)| (k, Arc::new(v))).collect();
    }

    /// Add all given elements to current data.
    #[inline(always)]
    pub fn add(&mut self, t: HashMap<String, VmTemplate>) {
        let mut ts = self.write();
        t.into_iter().for_each(|(k, v)| {
            ts.insert(k, Arc::new(v));
        })
    }

    /// Similar to `add`, but ensure none of existing templetes will be replaced.
    #[inline(always)]
    pub fn add_safe(&mut self, t: HashMap<String, VmTemplate>) -> Result<()> {
        if self.read().keys().any(|k| t.get(k).is_some()) {
            return crate::fail!(ERR_TT_CTRL_UPDATE_TEMPLATE);
        }
        self.add(t);
        Ok(())
    }

    /// Delete all given templates from current data.
    #[inline(always)]
    pub fn del(&mut self, t: HashSet<String>) {
        let mut ts = self.write();
        t.iter().for_each(|t| {
            ts.remove(t);
        })
    }

    /// Get an reference of `VmTemplate` from a given name.
    pub fn get(&self, name: &str) -> Option<Arc<VmTemplate>> {
        self.read().get(name).map(Arc::clone)
    }
}

impl Deref for TemplateCtrl {
    type Target = Arc<RwLock<HashMap<String, Arc<VmTemplate>>>>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// Total resources of the host.
#[derive(Clone)]
pub struct HardwareCtrl(Arc<Hardware>);

impl HardwareCtrl {
    fn init(total: Option<Hardware>) -> Option<HardwareCtrl> {
        static mut HW: Option<HardwareCtrl> = None;

        unsafe {
            if let Some(hw) = HW.as_ref() {
                Some(hw.clone())
            } else if let Some(hw) = total {
                let ret = HardwareCtrl(Arc::new(hw));
                HW = Some(ret.clone());
                Some(ret)
            } else {
                None
            }
        }
    }
}

impl Deref for HardwareCtrl {
    type Target = Arc<Hardware>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
