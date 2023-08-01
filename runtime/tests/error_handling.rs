// Licensed under the Apache-2.0 license.
pub mod common;
use caliptra_hw_model::HwModel;
use common::run_rt_test;

#[test]
fn test_wdt_timeout() {
    let mut model = run_rt_test(Some("wdt"));
    model.step_until(|m| m.soc_ifc().cptra_fw_error_fatal().read() != 0);

    // Make sure we see the right fatal error
    assert_eq!(model.soc_ifc().cptra_fw_error_fatal().read(), 0x0000_DEAD1);
}
