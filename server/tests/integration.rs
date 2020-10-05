//!
//! # Integration Tests.
//!

#![cfg(feature = "testmock")]

mod env;
mod knead;
mod standalone;

#[test]
fn i_ttserver() {
    if 0 == env::get_uid() {
        env::start_server();
        standalone::test();
        knead::test();
    } else {
        println!("\x1b[31;01mNOT root, ignore...\x1b[00m");
    }
}
