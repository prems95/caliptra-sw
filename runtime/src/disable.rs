// Licensed under the Apache-2.0 license

use crate::{Drivers, MailboxResp};
use caliptra_common::keyids::{KEY_ID_RT_CDI, KEY_ID_RT_PRIV_KEY};
use caliptra_drivers::{
    hmac384_kdf, CaliptraError, CaliptraResult, KeyReadArgs, KeyUsage, KeyWriteArgs,
};

pub struct DisableAttestationCmd;
impl DisableAttestationCmd {
    pub(crate) fn execute(drivers: &mut Drivers) -> CaliptraResult<MailboxResp> {
        drivers
            .key_vault
            .erase_key(KEY_ID_RT_CDI)
            .map_err(|_| CaliptraError::RUNTIME_DISABLE_ATTESTATION_FAILED)?;
        drivers
            .key_vault
            .erase_key(KEY_ID_RT_PRIV_KEY)
            .map_err(|_| CaliptraError::RUNTIME_DISABLE_ATTESTATION_FAILED)?;

        Self::generate_dice_key(drivers)?;
        Ok(MailboxResp::default())
    }

    // Dice key is derived from an empty CDI slot so it will not match the key that was certified in the rt_alias cert.
    fn generate_dice_key(drivers: &mut Drivers) -> CaliptraResult<()> {
        hmac384_kdf(
            &mut drivers.hmac384,
            KeyReadArgs::new(KEY_ID_RT_CDI).into(),
            b"dice_keygen",
            None,
            &mut drivers.trng,
            KeyWriteArgs::new(
                KEY_ID_RT_PRIV_KEY,
                KeyUsage::default()
                    .set_hmac_key_en()
                    .set_ecc_key_gen_seed_en(),
            )
            .into(),
        )?;

        Ok(())
    }
}
