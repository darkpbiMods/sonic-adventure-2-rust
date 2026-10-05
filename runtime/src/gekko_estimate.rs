// Copyright 2018 Dolphin Emulator Project
// SPDX-License-Identifier: GPL-2.0-or-later
// Positive-normal subset adapted from Dolphin 2609 Common/FloatUtils.cpp:
// https://github.com/dolphin-emu/dolphin/blob/2609/Source/Core/Common/FloatUtils.cpp
// Used to preserve the Gekko frsqrte seed before the game's Newton iterations.
// This helper's licensing is separate from the privately reconstructed game code.
pub(crate) fn reciprocal_sqrt_seed(value:f64)->f64 {
    assert!(value.is_normal() && value>0.);
    const TABLE:[(i64,i64);32]=[
        (0x1a7e800,-0x568),(0x17cb800,-0x4f3),(0x1552800,-0x48d),(0x130c000,-0x435),
        (0x10f2000,-0x3e7),(0x0eff000,-0x3a2),(0x0d2e000,-0x365),(0x0b7c000,-0x32e),
        (0x09e5000,-0x2fc),(0x0867000,-0x2d0),(0x06ff000,-0x2a8),(0x05ab800,-0x283),
        (0x046a000,-0x261),(0x0339800,-0x243),(0x0218800,-0x226),(0x0105800,-0x20b),
        (0x3ffa000,-0x7a4),(0x3c29000,-0x700),(0x38aa000,-0x670),(0x3572000,-0x5f2),
        (0x3279000,-0x584),(0x2fb7000,-0x524),(0x2d26000,-0x4cc),(0x2ac0000,-0x47e),
        (0x2881000,-0x43a),(0x2665000,-0x3fa),(0x2468000,-0x3c2),(0x2287000,-0x38e),
        (0x20c1000,-0x35e),(0x1f12000,-0x332),(0x1d79000,-0x30a),(0x1bf4000,-0x2e6),
    ];
    let bits=value.to_bits()as i64;
    let exponent=bits&(0x7ff_i64<<52);
    let mantissa=bits&((1_i64<<52)-1);
    let index=(((exponent&(1_i64<<52))|mantissa)>>37)as usize;
    let (base,delta)=TABLE[index/2048];
    let result_exp=((0x3ff_i64<<52)-((exponent-(0x3fe_i64<<52))/2))&(0x7ff_i64<<52);
    f64::from_bits((result_exp|((base+delta*(index%2048)as i64)<<26))as u64)
}

/// Positive finite/infinite subset of Dolphin's Gekko fres estimate.
pub(crate) fn reciprocal_seed(value:f64)->f64 {
    assert!(value>0.);
    if value.is_infinite(){return 0.;}
    const TABLE:[(u64,u64);32]=[
        (0x7ff800,0x3e1),(0x783800,0x3a7),(0x70ea00,0x371),(0x6a0800,0x340),
        (0x638800,0x313),(0x5d6200,0x2ea),(0x579000,0x2c4),(0x520800,0x2a0),
        (0x4cc800,0x27f),(0x47ca00,0x261),(0x430800,0x245),(0x3e8000,0x22a),
        (0x3a2c00,0x212),(0x360800,0x1fb),(0x321400,0x1e5),(0x2e4a00,0x1d1),
        (0x2aa800,0x1be),(0x272c00,0x1ac),(0x23d600,0x19b),(0x209e00,0x18b),
        (0x1d8800,0x17c),(0x1a9000,0x16e),(0x17ae00,0x15b),(0x14f800,0x15b),
        (0x124400,0x143),(0x0fbe00,0x143),(0x0d3800,0x12d),(0x0ade00,0x12d),
        (0x088400,0x11a),(0x065000,0x11a),(0x041c00,0x108),(0x020c00,0x106),
    ];
    let bits=value.to_bits();let exponent=bits&(0x7ff<<52);
    if exponent<(895<<52){return f32::MAX as f64;}
    if exponent>=(1149<<52){return 0.;}
    let i=((bits&((1<<52)-1))>>37)as usize;
    let (base,delta)=TABLE[i/1024];
    f64::from_bits(((0x7fd<<52)-exponent)|((base-(delta*(i%1024)as u64+1)/2)<<29))
}
