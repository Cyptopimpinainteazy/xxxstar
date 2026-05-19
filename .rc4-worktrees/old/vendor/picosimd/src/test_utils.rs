#![allow(dead_code)]

#[allow(unused_imports)]
pub(crate) use crate::indexes;

pub fn test_array_i32x32() -> [i32; 32] {
    [
        0x5668ba8f_u32 as i32,
        0xd8dff557_u32 as i32,
        0x27d0383a_u32 as i32,
        0xc5820461_u32 as i32,
        0x6fecdef0_u32 as i32,
        0x42b0ecb5_u32 as i32,
        0xea03afd3_u32 as i32,
        0x2c503a5a_u32 as i32,
        0x0f36e123_u32 as i32,
        0x13c3cdc2_u32 as i32,
        0x44144066_u32 as i32,
        0x4599a64c_u32 as i32,
        0x64476ac2_u32 as i32,
        0x42f2d7ba_u32 as i32,
        0x858b345b_u32 as i32,
        0x3bec794e_u32 as i32,
        0xa72353dc_u32 as i32,
        0x1a912fe7_u32 as i32,
        0xaa98ca29_u32 as i32,
        0xa65f4e0a_u32 as i32,
        0x7e68125b_u32 as i32,
        0xb0b5ef15_u32 as i32,
        0xfc097a32_u32 as i32,
        0x3337f33b_u32 as i32,
        0x10462bc8_u32 as i32,
        0x5db53959_u32 as i32,
        0x73c91ab3_u32 as i32,
        0xcf53c294_u32 as i32,
        0x83efd80a_u32 as i32,
        0xb44d8a8d_u32 as i32,
        0xda7f8331_u32 as i32,
        0x8fea8e08_u32 as i32,
    ]
}

pub fn test_array_i64x8() -> [i64; 8] {
    let array = test_array_i32x32();
    unsafe { array.as_ptr().cast::<[i64; 8]>().read() }
}

pub fn test_array_i64x4() -> [i64; 4] {
    let array = test_array_i32x32();
    unsafe { array.as_ptr().cast::<[i64; 4]>().read() }
}

pub fn test_array_i64x2() -> [i64; 2] {
    let array = test_array_i32x32();
    unsafe { array.as_ptr().cast::<[i64; 2]>().read() }
}

pub fn test_array_i32x16() -> [i32; 16] {
    let array = test_array_i32x32();
    unsafe { array.as_ptr().cast::<[i32; 16]>().read() }
}

pub fn test_array_i32x8() -> [i32; 8] {
    let array = test_array_i32x32();
    unsafe { array.as_ptr().cast::<[i32; 8]>().read() }
}

pub fn test_array_i32x4() -> [i32; 4] {
    let array = test_array_i32x32();
    unsafe { array.as_ptr().cast::<[i32; 4]>().read() }
}

pub fn test_array_i16x32() -> [i16; 32] {
    let array = test_array_i32x32();
    unsafe { array.as_ptr().cast::<[i16; 32]>().read() }
}

pub fn test_array_i16x16() -> [i16; 16] {
    let array = test_array_i32x32();
    unsafe { array.as_ptr().cast::<[i16; 16]>().read() }
}

pub fn test_array_i8x64() -> [i8; 64] {
    let array = test_array_i32x32();
    unsafe { array.as_ptr().cast::<[i8; 64]>().read() }
}

pub fn test_array_i8x32() -> [i8; 32] {
    let array = test_array_i32x32();
    unsafe { array.as_ptr().cast::<[i8; 32]>().read() }
}
