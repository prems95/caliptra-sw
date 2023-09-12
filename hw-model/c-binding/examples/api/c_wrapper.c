#include "c_wrapper.h"
#include <stdint.h>
#include <stdlib.h>
#include <stdio.h>
#include <string.h>
#include <errno.h>


uint32_t calculate_caliptra_checksum(uint32_t cmd, uint8_t *buffer, uint32_t len)
{
       printf("cmd = %d\n", cmd);
      fflush(stdout);
  for (uint32_t i = 0; i < len; i++) {
        printf("%02X ", buffer[i]); // Print each byte as a 2-digit hexadecimal number
         fflush(stdout);
    }
    printf("\n");
  fflush(stdout);
printf("len = %d\n", len);
  fflush(stdout);
    uint32_t i, sum = 0;

    if ((buffer == NULL) && (len != 0))
    {
        // Don't respect bad parameters
        return 0;
    }

    for (i = 0; i < sizeof(uint32_t); i++)
    {
        sum += ((uint8_t*)(&cmd))[i];
    }

    for (i = 0; i < len; i++)
    {
        sum += buffer[i];
    }

    return (0 - sum);
}

void create_command_hdr(uint32_t magic, uint32_t cmd, uint32_t profile, CommandHdr* cmdHdr) {
    
    if (cmdHdr != NULL) {
        cmdHdr->magic = magic;
        cmdHdr->cmd = cmd;
        cmdHdr->profile = profile;
    }
}

// Function to print the struct as bytes
void printStructAsBytes(CommandHdr* cmdHdr) {
    // Ensure cmdHdr is not NULL
    if (cmdHdr != NULL) {
        // Convert the struct to a byte array
        uint8_t bytes[sizeof(CommandHdr)];
        memcpy(bytes, cmdHdr, sizeof(CommandHdr));

        // Print the byte array
        for (int i = 0; i < sizeof(CommandHdr); i++) {
            printf("%02X ", bytes[i]);
            fflush(stdout);
        }
        printf("\n");
        fflush(stdout);
    }
}


caliptra_buffer create_invoke_dpe_command(uint32_t magic, uint32_t cmd, uint32_t profile) {
    printf("****INVOKE 1**********\n");
    fflush(stdout);
    CommandHdr cmdHdr;
    create_command_hdr(magic, cmd, profile, &cmdHdr);
    printf("****INVOKE 2**********\n");
    fflush(stdout);

    uint32_t data_size = sizeof(CommandHdr);
    const uint32_t OP_INVOKE_DPE_COMMAND = 0x44504543;
    // printf("%u\n", calculate_caliptra_checksum(OP_INVOKE_DPE_COMMAND, (uint8_t*)&cmdHdr, data_size));

    INVOKE_DPE_COMMAND* invokeCmd = (INVOKE_DPE_COMMAND*)malloc(sizeof(INVOKE_DPE_COMMAND));
    if (invokeCmd != NULL) {
        printf("****INVOKE 3**********\n");
        memset(invokeCmd, 0, sizeof(INVOKE_DPE_COMMAND));
        fflush(stdout);
        invokeCmd->data_size = data_size;
        printf("%d\n",data_size);
        fflush(stdout);
         // Copy cmdHdr data into invokeCmd->data
        memcpy(invokeCmd->data, &cmdHdr, data_size);
        invokeCmd->chksum = calculate_caliptra_checksum(OP_INVOKE_DPE_COMMAND, (uint8_t*)invokeCmd, sizeof(INVOKE_DPE_COMMAND) - sizeof(uint32_t));
        printf("*********INVOKE 4************\n");
        fflush(stdout);
    }

    printf("Checksum : %x\n", invokeCmd->chksum);
    printStructAsBytes(&cmdHdr);
    printf("data size : %x\n", invokeCmd->data_size);

    caliptra_buffer buffer = {
        .data = (const uint8_t*)invokeCmd,
        .len = sizeof(INVOKE_DPE_COMMAND)
    };

    return buffer;
}