// Copyright 2009 Dolphin Emulator Project
// SPDX-License-Identifier: GPL-2.0-or-later
// Normal-value subset of Force25Bit from Dolphin 2609:
// https://github.com/dolphin-emu/dolphin/blob/2609/Source/Core/Core/PowerPC/Interpreter/Interpreter_FPUtils.h
// fmuls rounds its C operand to 25 significand bits before multiplication.
pub(crate) fn normal_25_bit(value:f64)->f64 {
    assert!(value.is_normal());
    let bits=value.to_bits();
    f64::from_bits((bits&0xffff_ffff_f800_0000)+(bits&0x0800_0000))
}
