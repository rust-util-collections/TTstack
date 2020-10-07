//!
//! A fast implementation based on ttrexec.
//!

use super::super::{run::VmConnInfo, POOL};
use myutil::{err::*, *};
use std::sync::mpsc::{channel, Receiver};
use ttrexec::{
    client::req_transfer,
    common::{Direction, TransReq},
};

pub(super) fn exec(
    file_path: &str,
    vm_conn_info: Vec<VmConnInfo>,
) -> Receiver<VmConnInfo> {
    let (s, r) = channel();

    vm_conn_info.into_iter().for_each(|mut vci| {
        let fpath = file_path.to_owned();
        let sender = s.clone();
        POOL.spawn(move || {
            TransReq::new(
                Direction::Push,
                &fpath,
                &format!("/tmp/{}", fpath.rsplitn(2, '/').next().unwrap()),
            )
            .and_then(|req| {
                let addr = format!("{}:{}", vci.addr, vci.ttrexec_port);
                let resp = req_transfer(&addr, req, None).c(d!())?;
                vci.stdout = resp.stdout.into_owned();
                vci.stderr = resp.stderr.into_owned();
                Ok(())
            })
            .unwrap_or_else(|e| {
                vci.stderr = genlog(e);
            });

            info_omit!(sender.send(vci));
        });
    });

    r
}
