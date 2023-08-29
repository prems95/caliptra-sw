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
    "flag"
    "os"
    "unsafe"
	"fmt"
)

type CommandHdr struct {
	Magic   uint32
	Cmd     CommandCode
	Profile Profile
}

type Profile struct {
	MajorVersion uint16
	MinorVersion uint16
}

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

func main() {
    // Process Input Arguments
    romPath := flag.String("r", "", "rom file path")
    fwPath := flag.String("f", "", "fw image file path")
    flag.Parse()

    if *romPath == "" || *fwPath == "" {
        flag.Usage()
        os.Exit(int(C.EINVAL))
    }

    // Initialize Params
    initParams := C.caliptra_model_init_params{
        rom: read_file_or_die(C.CString(*romPath)),
        dccm: C.caliptra_buffer{data: nil, len: 0},
        iccm: C.caliptra_buffer{data: nil, len: 0},
    }

    // Initialize Model
    var model *C.caliptra_model
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
    imageBundle := read_file_or_die(C.CString(*fwPath))
    C.caliptra_upload_fw(model, &imageBundle)

    // Run Until RT is ready to receive commands
    for {
    C.caliptra_model_step(model)
        buffer := C.caliptra_model_output_peek(model)
        if C.strstr((*C.char)(unsafe.Pointer(buffer.data)), C.CString("Caliptra RT listening for mailbox commands...")) != nil {
            var test C.uint32_t
            profileBuffer := C.create_invoke_dpe_command(C.uint32_t(CmdMagic), C.uint32_t(CommandGetProfile),C.uint32_t(0x1))
            fmt.Println(profileBuffer)
            var Check C.caliptra_output
            var profile C.int
            profile = 5
            profile = C.caliptra_get_profile(model, &profileBuffer,test,&Check)
            fmt.Println("***********Status***************:\n",profile)
            fmt.Println(test)
            fmt.Println(Check)
            break
        }
    }
    fmt.Println("Caliptra C Smoke Test Passed \n")
}

func main() {
    // Read serialized data from a file or standard input
    serializedData, err := read_serialized_data() // Implement this function to read the data
    if err != nil {
        fmt.Printf("Error reading serialized data: %v\n", err)
        os.Exit(1)
    }

    // Connect to the running executable via Unix socket
    socketPath := "/tmp/test.socket" // Provide the actual path to the Unix socket
    conn, err := net.Dial("unix", socketPath)
    if err != nil {
        fmt.Printf("Error connecting to socket: %v\n", err)
        os.Exit(1)
    }
    defer conn.Close()

    // Send serialized data to the executable
    _, err = conn.Write(serializedData)
    if err != nil {
        fmt.Printf("Error sending data to executable: %v\n", err)
        os.Exit(1)
    }

    // Receive and print the response from the executable
    response, err := io.ReadAll(conn)
    if err != nil {
        fmt.Printf("Error reading response from executable: %v\n", err)
        os.Exit(1)
    }
    fmt.Printf("Response from executable: %s\n", response)
}