// Licensed under the Apache-2.0 license
#ifndef CALIPTRA_API_H
#define CALIPTRA_API_H

#include <stdint.h>
#include "caliptra_model.h"

// Initialize Caliptra fuses prior to boot
int caliptra_init_fuses(struct caliptra_model *model, struct caliptra_fuses *fuses);

// Write into Caliptra BootFSM Go Register
int caliptra_bootfsm_go(struct caliptra_model *model);

// Upload Caliptra Firmware
int caliptra_upload_fw(struct caliptra_model *model, struct caliptra_buffer *fw_buffer);

int caliptra_get_profile(struct caliptra_model *model, struct caliptra_buffer *fw_buffer,uint32_t statusCheckRead);

// Execute Mailbox Command
int caliptra_mailbox_execute(struct caliptra_model *model, uint32_t cmd, struct caliptra_buffer *mbox_tx_buffer, struct caliptra_buffer *mbox_rx_buffer);

#endif // CALIPTRA_API_H