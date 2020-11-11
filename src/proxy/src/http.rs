//!
//! # Http Interfaces
//!
//! Commiunication with client-end.
//!

use crate::{hdr, util, DEFAULT_REQ_ID, RECV_TO_SECS, UAU_ID};
use async_std::future;
use myutil::{err::*, *};
use std::{sync::atomic::Ordering, time::Duration};
use tide::{Body, Error, Request};

macro_rules! err {
    (@$e: expr) => {
        Error::from_str(500, util::gen_resp_err(DEFAULT_REQ_ID, &genlog($e)))
    };
    ($e: expr) => {
        Err(err!(@$e))
    };
}

macro_rules! gen_hdr {
    ($ops: ident, $idx: expr) => {
        pub(super) async fn $ops(
            mut req: Request<()>,
        ) -> tide::Result<Body> {
            const OPS_ID: usize = $idx;
            let msg = req.body_bytes().await?;
            let uau_addr = UAU_ID.fetch_add(1, Ordering::Relaxed).to_ne_bytes();

            let (mysock, myaddr) = match util::gen_uau_socket(&uau_addr) {
                Ok((s, a)) => (s, a),
                Err(e) => return err!(e),
            };

            hdr::OPS_MAP[OPS_ID](OPS_ID, myaddr, msg)
                .c(d!())
                .map_err(|e| err!(@e))?;

            let mut buf = vec![0; 128 * 1024];
            future::timeout(Duration::from_secs(RECV_TO_SECS), mysock.recv(&mut buf))
                .await
                .c(d!())
                .map_err(|e| err!(@e))?
                .c(d!())
                .map(|siz| {
                    unsafe {
                        buf.set_len(siz);
                    }
                    Body::from_bytes(buf)
                })
                .map_err(|e| err!(@e))
        }
    }
}

gen_hdr!(register_client_id, 0);
gen_hdr!(get_server_info, 1);
gen_hdr!(get_env_list, 2);
gen_hdr!(get_env_info, 3);
gen_hdr!(add_env, 4);
gen_hdr!(del_env, 5);
gen_hdr!(update_env_lifetime, 6);
gen_hdr!(update_env_kick_vm, 7);
gen_hdr!(get_env_list_all, 8);
gen_hdr!(stop_env, 9);
gen_hdr!(start_env, 10);
gen_hdr!(update_env_resource, 11);
