package main

// #cgo CFLAGS: -I./../out/debug -std=c99
// #cgo LDFLAGS: -L./../out/debug -lcaliptra_hw_model_c_binding -lc_wrapper -ldl
// #include "../../../../hw-latest/caliptra-rtl/src/soc_ifc/rtl/caliptra_top_reg.h"
// #include "caliptra_api.h"
// #include "caliptra_fuses.h"
// #include "caliptra_mbox.h"
// #include "c_wrapper.h"
// #include <stdlib.h>
// #include <stdio.h>
// #include <stdint.h>
// #include <string.h>
// #include <errno.h>
// #include <unistd.h>
// extern int caliptra_mailbox_write_fifo(struct caliptra_model *model, struct caliptra_buffer *buffer);
import "C"

import (
    "os"
    "unsafe"
	"fmt"
)

type CommandHdr struct {
	Magic   uint32
	Cmd     CommandCode
	Profile Profile
}

// Initialize Model
var model *C.caliptra_model

type CommandCode uint32

const (
	CommandGetProfile        CommandCode = 0x1
	CommandInitializeContext CommandCode = 0x7
	CommandCertifyKey        CommandCode = 0x9
	CommandDestroyContext    CommandCode = 0xf
	CommandTagTCI            CommandCode = 0x82
	CommandGetTaggedTCI      CommandCode = 0x83
)

const (
	CmdMagic  uint32 = 0x44504543
)

func read_file_or_die(path *C.char) C.caliptra_buffer {
    // Open File in Read Only Mode
    fp := C.fopen(path, C.CString("r"))
    if fp == nil {
        fmt.Printf("Cannot find file %s \n", path)
        os.Exit(int(C.ENOENT))
    }

    var buffer C.caliptra_buffer

    // Get File Size
    C.fseek(fp, 0, C.SEEK_END)
    buffer.len = C.ulong(C.ftell(fp))
    C.fseek(fp, 0, C.SEEK_SET)

    // Allocate Buffer Memory
    buffer.data = (*C.uchar)(C.malloc(C.size_t(buffer.len)))
    if buffer.data == nil {
        fmt.Println("Cannot allocate memory for buffer->data \n")
        os.Exit(int(C.ENOMEM))
    }

    // Read Data in Buffer
    C.fread(unsafe.Pointer(buffer.data), C.size_t(buffer.len), 1, fp)

    return buffer
}

func Start() {
    // Process Input Arguments
    //romPath := flag.String("r", "", "rom file path")
   // fwPath := flag.String("f", "", "fw image file path")
    //flag.Parse()

   /* if *romPath == "" || *fwPath == "" {
        flag.Usage()
        os.Exit(int(C.EINVAL))
    }*/

    romPath := "../out/caliptra_rom.bin"
    fwPath := "../out/image_bundle.bin"

    // Initialize Params
    initParams := C.caliptra_model_init_params{
        rom: read_file_or_die(C.CString(romPath)),
        dccm: C.caliptra_buffer{data: nil, len: 0},
        iccm: C.caliptra_buffer{data: nil, len: 0},
    }

    C.caliptra_model_init_default(initParams, &model)

    // Initialize Fuses (Todo: Set real fuse values)
    var fuses C.caliptra_fuses
    C.caliptra_init_fuses(model, &fuses)

    // Initialize FSM GO
    C.caliptra_bootfsm_go(model)
    C.caliptra_model_step(model)

    // Step until ready for FW
    for (C.caliptra_model_ready_for_fw(model)) {
        C.caliptra_model_step(model)
    }

    // Load Image Bundle
    imageBundle := read_file_or_die(C.CString(fwPath))
    C.caliptra_upload_fw(model, &imageBundle)

    // Run Until RT is ready to receive commands
    for {
    C.caliptra_model_step(model)
        buffer := C.caliptra_model_output_peek(model)
        if C.strstr((*C.char)(unsafe.Pointer(buffer.data)), C.CString("Caliptra RT listening for mailbox commands...")) != nil {
            break
        }
    }
    fmt.Println("Caliptra C Smoke Test Passed \n")
}

func Commands(bytes []byte){
    var test C.uint32_t
     // Convert the []byte to a *C.uchar pointer
     cBytes := (*C.uint8_t)(unsafe.Pointer(&bytes[0])) // Pointer to the first element
     length := C.size_t(len(bytes))
 
     // Convert the length to uint32_t using type conversion
     dataSize := C.uint32_t(length)
    profileBuffer := C.create_invoke_dpe_command(cBytes,dataSize)
    fmt.Println(profileBuffer)
    var Check C.caliptra_output
    var profile C.int
    profile = 5
    profile = C.caliptra_get_profile(model, &profileBuffer,test,&Check)
    fmt.Println("***********Status***************:\n",profile)
    fmt.Println(test)
    fmt.Println(Check)
}