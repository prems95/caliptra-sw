/*++

Licensed under the Apache-2.0 license.

Inspired by OpenTitan's driver interface functions for the entropy_src and
CSRNG peripherals:
https://opentitan.org/book/sw/device/lib/dif/dif_entropy_src_h.html
https://opentitan.org/book/sw/device/lib/dif/dif_csrng_h.html

An overview of the entropy_src and CSRNG peripherals can be found at:
https://opentitan.org/book/hw/ip/entropy_src/index.html
https://opentitan.org/book/hw/ip/csrng/index.html

File Name:

    csrng.rs

Abstract:

    Software interface to the Cryptographically Secure Random Number Generator
    (CSRNG) peripheral.

--*/
use crate::{wait, CaliptraError, CaliptraResult};
use caliptra_registers::{csrng::CsrngReg, entropy_src::regs::AlertFailCountsReadVal};
use core::{iter::FusedIterator, num::NonZeroUsize};

// https://opentitan.org/book/hw/ip/csrng/doc/theory_of_operation.html#command-description
const MAX_SEED_WORDS: usize = 12;
const MAX_GENERATE_BLOCKS: usize = 4096;
const WORDS_PER_GENERATE_BLOCK: usize = 4;

/// A unique handle to the underlying CSRNG peripheral.
pub struct Csrng {
    csrng: caliptra_registers::csrng::CsrngReg,
    entropy_src: caliptra_registers::entropy_src::EntropySrcReg,
}

impl Csrng {
    /// Returns a handle to the CSRNG in TRNG mode.
    ///
    /// The CSRNG will gather seed material from the entropy_src peripheral.
    ///
    /// # Safety
    ///
    /// No other handles to the CSRNG should exist.
    ///
    /// # Errors
    ///
    /// Returns an error if the internal seed command fails.
    pub fn new(
        csrng: caliptra_registers::csrng::CsrngReg,
        entropy_src: caliptra_registers::entropy_src::EntropySrcReg,
    ) -> CaliptraResult<Self> {
        Self::with_seed(csrng, entropy_src, Seed::EntropySrc)
    }

    /// # Safety
    ///
    /// The caller MUST ensure that the CSRNG peripheral is in a state where new
    /// entropy is accessible via the generate command.
    pub unsafe fn assume_initialized(
        csrng: caliptra_registers::csrng::CsrngReg,
        entropy_src: caliptra_registers::entropy_src::EntropySrcReg,
    ) -> Self {
        Self { csrng, entropy_src }
    }

    /// Returns a handle to the CSRNG configured to use the provided [`Seed`].
    ///
    /// # Safety
    ///
    /// No other handles to the CSRNG should exist.
    ///
    /// # Errors
    ///
    /// Returns an error if the internal seed command fails.
    pub fn with_seed(
        csrng: caliptra_registers::csrng::CsrngReg,
        entropy_src: caliptra_registers::entropy_src::EntropySrcReg,
        seed: Seed,
    ) -> CaliptraResult<Self> {
        const FALSE: u32 = MultiBitBool::False as u32;
        const TRUE: u32 = MultiBitBool::True as u32;

        // Configure and enable entropy_src if needed.

        let mut result = Self { csrng, entropy_src };
        let c = result.csrng.regs_mut();
        let e = result.entropy_src.regs_mut();

        if e.module_enable().read().module_enable() == FALSE {
            e.conf()
                .write(|w| w.fips_enable(FALSE).entropy_data_reg_enable(FALSE));
            e.module_enable().write(|w| w.module_enable(TRUE));
            wait::until(|| e.debug_status().read().main_sm_boot_done());
        }

        if c.ctrl().read().enable() == FALSE {
            c.ctrl()
                .write(|w| w.enable(TRUE).sw_app_enable(TRUE).read_int_state(TRUE));
        }

        send_command(&mut result.csrng, Command::Uninstantiate)?;
        send_command(&mut result.csrng, Command::Instantiate(seed))?;

        Ok(result)
    }

    /// Returns an iterator over `num_words` random [`u32`]s.
    ///
    /// This function will round up to the nearest multiple of four words.
    ///
    /// # Errors
    ///
    /// Returns an error if the internal generate command fails.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// let mut csrng = ...;
    ///
    /// let num_words = NonZeroUsize::new(1).unwrap();
    /// let mut random_words = csrng.generate(num_words)?;
    ///
    /// // Rounds up to nearest multiple of four.
    /// assert_eq!(random_words.len(), 4);
    ///
    /// for word in random_words {
    ///     // Do something with `word`.
    /// }
    /// ```
    pub fn generate(&mut self, num_words: NonZeroUsize) -> CaliptraResult<Iter> {
        // Round up to nearest multiple of 128-bit block.
        let num_128_bit_blocks = (num_words.get() + 3) / 4;
        let num_words = num_128_bit_blocks * WORDS_PER_GENERATE_BLOCK;

        send_command(&mut self.csrng, Command::Generate { num_128_bit_blocks })?;

        Ok(Iter {
            csrng: &mut self.csrng,
            num_words_left: num_words,
        })
    }

    pub fn reseed(&mut self, seed: Seed) -> CaliptraResult<()> {
        send_command(&mut self.csrng, Command::Reseed(seed))
    }

    pub fn update(&mut self, additional_data: &[u32]) -> CaliptraResult<()> {
        send_command(&mut self.csrng, Command::Update(additional_data))
    }

    /// Returns the number of failing health checks.
    pub fn health_counts(&self) -> HealthFailCounts {
        let e = self.entropy_src.regs();

        HealthFailCounts {
            total: e.alert_summary_fail_counts().read().any_fail_count(),
            specific: e.alert_fail_counts().read(),
        }
    }

    pub fn uninstantiate(mut self) {
        let _ = send_command(&mut self.csrng, Command::Uninstantiate);
    }
}

/// Variants that describe seed inputs to the CSRNG.
pub enum Seed<'a> {
    /// Use a non-deterministic seed.
    EntropySrc,

    /// Use a deterministic seed. The number of seed words should be at least
    /// one and no more than twelve.
    Constant(&'a [u32]),
}

enum Command<'a> {
    Instantiate(Seed<'a>),
    Reseed(Seed<'a>),
    Generate { num_128_bit_blocks: usize },
    Update(&'a [u32]),
    Uninstantiate,
}

#[repr(u32)]
enum MultiBitBool {
    False = 9,
    True = 6,
}

/// An iterator over random [`u32`]s.
///
/// This struct is created by the [`generate`] method on [`Csrng`].
///
/// [`generate`]: Csrng::generate
pub struct Iter<'a> {
    // It's not clear what reseeding or updating the CSRNG state would do
    // to an existing generate request. Prevent these operations from happening
    // concurrent to this iterator's life.
    csrng: &'a mut CsrngReg,
    num_words_left: usize,
}

impl Iterator for Iter<'_> {
    type Item = u32;

    fn next(&mut self) -> Option<Self::Item> {
        let csrng = self.csrng.regs();
        if self.num_words_left == 0 {
            None
        } else {
            if self.num_words_left % WORDS_PER_GENERATE_BLOCK == 0 {
                // Wait for CSRNG to generate next block of 4 words.
                wait::until(|| csrng.genbits_vld().read().genbits_vld());
            }

            self.num_words_left -= 1;

            Some(csrng.genbits().read())
        }
    }
}

impl ExactSizeIterator for Iter<'_> {
    fn len(&self) -> usize {
        self.num_words_left
    }
}

impl FusedIterator for Iter<'_> {}

impl Drop for Iter<'_> {
    fn drop(&mut self) {
        // Exhaust this generate request.
        for _ in self {}
    }
}

/// Contains counts of failing health checks.
///
/// This struct is returned by the [`health_counts`] method on [`Csrng`].
///
/// [`health_counts`]: Csrng::health_counts
pub struct HealthFailCounts {
    /// The total number of failing health check alerts.
    pub total: u32,

    /// The counts of specific failing health checks.
    pub specific: AlertFailCountsReadVal,
}

fn send_command(csrng: &mut CsrngReg, command: Command) -> CaliptraResult<()> {
    // https://opentitan.org/book/hw/ip/csrng/doc/theory_of_operation.html#general-command-format
    let acmd: u32;
    let clen: usize;
    let flag0: MultiBitBool;
    let glen: usize;
    let extra_words: &[u32];
    let err: CaliptraError;

    match command {
        Command::Instantiate(ref seed) | Command::Reseed(ref seed) => {
            acmd = if matches!(command, Command::Instantiate(_)) {
                err = CaliptraError::DRIVER_CSRNG_INSTANTIATE;
                1
            } else {
                err = CaliptraError::DRIVER_CSRNG_RESEED;
                2
            };

            match seed {
                Seed::EntropySrc => {
                    clen = 0;
                    flag0 = MultiBitBool::False;
                    extra_words = &[];
                }

                Seed::Constant(constant) => {
                    clen = constant.len().min(MAX_SEED_WORDS);
                    flag0 = MultiBitBool::True;
                    extra_words = &constant[..clen];
                }
            }

            glen = 0;
        }

        Command::Generate { num_128_bit_blocks } => {
            acmd = 3;
            clen = 0;
            flag0 = MultiBitBool::False;
            glen = num_128_bit_blocks.min(MAX_GENERATE_BLOCKS);
            extra_words = &[];
            err = CaliptraError::DRIVER_CSRNG_GENERATE;
        }

        Command::Update(words) => {
            acmd = 4;
            clen = words.len().min(MAX_SEED_WORDS);
            flag0 = MultiBitBool::True;
            glen = 0;
            extra_words = &words[..clen];
            err = CaliptraError::DRIVER_CSRNG_UPDATE;
        }

        Command::Uninstantiate => {
            acmd = 5;
            clen = 0;
            flag0 = MultiBitBool::False;
            glen = 0;
            extra_words = &[];
            err = CaliptraError::DRIVER_CSRNG_UNINSTANTIATE;
        }
    }

    let acmd = acmd & 0xf;
    let clen = (clen as u32) & 0xf;
    let flag0 = (flag0 as u32) & 0xf;
    let glen = (glen as u32) & 0x1fff;

    // Write mandatory 32-bit command header.
    csrng
        .regs_mut()
        .cmd_req()
        .write(|_| ((glen << 12) | (flag0 << 8) | (clen << 4) | acmd).into());

    // Write optional extra words.
    for &word in extra_words {
        csrng.regs_mut().cmd_req().write(|_| word.into());
    }

    // Wait for command.
    loop {
        let reg = csrng.regs().sw_cmd_sts().read();

        // Order matters. Check for errors first.
        if reg.cmd_sts() {
            // TODO(rkr35): Somehow convey additional error information found in
            // the ERR_CODE register.
            return Err(err);
        }

        if reg.cmd_rdy() {
            return Ok(());
        }
    }
}
