use crate::cpu::*;
use crate::utils::*;

const OPCODES: [fn(&mut Cpu) -> u8; 256] = [
//  0x00,    0x01,    0x02,    0x03,    0x04,    0x05,    0x06,    0x07,    0x08,    0x09,    0x0A,    0x0B,    0x0C,    0x0D,    0x0E,    0x0F
    nop_00,  ld_01,   ld_02,   inc_03,  inc_04,  dec_05,  ld_06,   rlca_07, ld_08,   add_09,  ld_0a,   dec_0b,  inc_0c,  dec_0d,  ld_0e,   rrca_0f, // 0x00
    stop_10, ld_11,   ld_12,   inc_13,  inc_14,  dec_15,  ld_16,   rla_17,  jr_18,   add_19,  ld_1a,   dec_1b,  inc_1c,  dec_1d,  ld_1e,   rra_1f,  // 0x10
    jr_20,   ld_21,   ld_22,   inc_23,  inc_24,  dec_25,  ld_26,   daa_27,  jr_28,   add_29,  ld_2a,   dec_2b,  inc_2c,  dec_2d,  ld_2e,   cpl_2f,  // 0x20
    jr_30,   ld_31,   ld_32,   inc_33,  inc_34,  dec_35,  ld_36,   scf_37,  jr_38,   add_39,  ld_3a,   dec_3b,  inc_3c,  dec_3d,  ld_3e,   ccf_3f,  // 0x30
    ld_40,   ld_41,   ld_42,   ld_43,   ld_44,   ld_45,   ld_46,   ld_47,   ld_48,   ld_49,   ld_4a,   ld_4b,   ld_4c,   ld_4d,   ld_4e,   ld_4f,   // 0x40
    ld_50,   ld_51,   ld_52,   ld_53,   ld_54,   ld_55,   ld_56,   ld_57,   ld_58,   ld_59,   ld_5a,   ld_5b,   ld_5c,   ld_5d,   ld_5e,   ld_5f,   // 0x50
    ld_60,   ld_61,   ld_62,   ld_63,   ld_64,   ld_65,   ld_66,   ld_67,   ld_68,   ld_69,   ld_6a,   ld_6b,   ld_6c,   ld_6d,   ld_6e,   ld_6f,   // 0x60
    ld_70,   ld_71,   ld_72,   ld_73,   ld_74,   ld_75,   halt_76, ld_77,   ld_78,   ld_79,   ld_7a,   ld_7b,   ld_7c,   ld_7d,   ld_7e,   ld_7f,   // 0x70
    add_80,  add_81,  add_82,  add_83,  add_84,  add_85,  add_86,  add_87,  add_88,  add_89,  add_8a,  add_8b,  add_8c,  add_8d,  add_8e,  add_8f,  // 0x80
    sub_90,  sub_91,  sub_92,  sub_93,  sub_94,  sub_95,  sub_96,  sub_97,  sbc_98,  sbc_99,  sbc_9a,  sbc_9b,  sbc_9c,  sbc_9d,  sbc_9e,  sbc_9f,  // 0x90
    and_a0,  and_a1,  and_a2,  and_a3,  and_a4,  and_a5,  and_a6,  and_a7,  xor_a8,  xor_a9,  xor_aa,  xor_ab,  xor_ac,  xor_ad,  xor_ae,  xor_af,  // 0xA0
    or_b0,   or_b1,   or_b2,   or_b3,   or_b4,   or_b5,   or_b6,   or_b7,   cp_b8,   cp_b9,   cp_ba,   cp_bb,   cp_bc,   cp_bd,   cp_be,   cp_bf,   // 0xB0
    ret_c0,  pop_c1,  jp_c2,   jp_c3,   call_c4, push_c5, add_c6,  rst_c7,  ret_c8,  ret_c9,  jp_ca,   prefix_cb, call_cc, call_cd, adc_ce,  rst_cf,  // 0xC0
    ret_d0,  pop_d1,  jp_d2,   invalid, call_d4, push_d5, sub_d6,  rst_d7,  ret_d8,  reti_d9, jp_da,   invalid, call_dc, invalid, sbc_de,  rst_df,  // 0xD0
    ld_e0,   pop_e1,  ld_e2,   invalid, invalid, push_e5, and_e6,  rst_e7,  add_e8,  jp_e9,   ld_ea,   invalid, invalid, invalid, xor_ee,  rst_ef,  // 0xE0
    ld_f0,   pop_f1,  ld_f2,   di_f3,   invalid, push_f5, or_f6,   rst_f7,  ld_f8,   ld_f9,   ld_fa,   ei_fb,   invalid, invalid, cp_fe,   rst_ff,  // 0xF0
];

pub fn execute(cpu: &mut Cpu) -> u8 {
    let op_index = cpu.fetch();
    OPCODES[op_index as usize](cpu)
}

fn prefix_cb(cpu: &mut Cpu) -> u8 {
    let cb_index = cpu.fetch();
    execute_cb(cpu, cb_index)
}

fn get_cb_reg(op: u8) -> Regs {
    match op & 0b111 {
        0 => Regs::B,
        1 => Regs::C,
        2 => Regs::D,
        3 => Regs::E,
        4 => Regs::H,
        5 => Regs::L,
        6 => Regs::HL,
        7 => Regs::A,
        _ => unreachable!(),
    }
}

fn execute_cb(cpu: &mut Cpu, op: u8) -> u8 {
    let cb_reg = get_cb_reg(op);
    let is_hl = cb_reg == Regs::HL;

    match op {
        0x00..=0x07 => cpu.rotate_left(cb_reg, false),
        0x08..=0x0F => cpu.rotate_right(cb_reg, false),
        0x10..=0x17 => cpu.rotate_left(cb_reg, true),
        0x18..=0x1F => cpu.rotate_right(cb_reg, true),
        0x20..=0x27 => cpu.shift_left(cb_reg),
        0x28..=0x2F => cpu.shift_right(cb_reg, true),
        0x30..=0x37 => cpu.swap_bits(cb_reg),
        0x38..=0x3F => cpu.shift_right(cb_reg, false),
        0x40..=0x7F => {
            let bit = (op & 0b0011_1000) >> 3;
            cpu.test_bit(cb_reg, bit);
            return if is_hl { 3 } else { 2 };
        }
        0x80..=0xBF => {
            let bit = (op & 0b0011_1000) >> 3;
            cpu.write_bit(cb_reg, bit, false);
        }
        0xC0..=0xFF => {
            let bit = (op & 0b0011_1000) >> 3;
            cpu.write_bit(cb_reg, bit, true);
        }
    }
    if is_hl { 4 } else { 2 }
}

fn nop_00(_cpu: &mut Cpu) -> u8 { 1 }

fn inc_03(cpu: &mut Cpu) -> u8 { cpu.inc_r16(Regs16::BC); cpu.internal_delay(); 2 }
fn inc_13(cpu: &mut Cpu) -> u8 { cpu.inc_r16(Regs16::DE); cpu.internal_delay(); 2 }
fn inc_23(cpu: &mut Cpu) -> u8 { cpu.inc_r16(Regs16::HL); cpu.internal_delay(); 2 }
fn inc_33(cpu: &mut Cpu) -> u8 { cpu.inc_r16(Regs16::SP); cpu.internal_delay(); 2 }

fn inc_04(cpu: &mut Cpu) -> u8 { cpu.inc_r8(Regs::B); 1 }
fn inc_14(cpu: &mut Cpu) -> u8 { cpu.inc_r8(Regs::D); 1 }
fn inc_24(cpu: &mut Cpu) -> u8 { cpu.inc_r8(Regs::H); 1 }
fn inc_34(cpu: &mut Cpu) -> u8 { cpu.inc_r8(Regs::HL); 3 }

fn inc_0c(cpu: &mut Cpu) -> u8 { cpu.inc_r8(Regs::C); 1 }
fn inc_1c(cpu: &mut Cpu) -> u8 { cpu.inc_r8(Regs::E); 1 }
fn inc_2c(cpu: &mut Cpu) -> u8 { cpu.inc_r8(Regs::L); 1 }
fn inc_3c(cpu: &mut Cpu) -> u8 { cpu.inc_r8(Regs::A); 1 }

fn dec_05(cpu: &mut Cpu) -> u8 { cpu.dec_r8(Regs::B); 1 }
fn dec_15(cpu: &mut Cpu) -> u8 { cpu.dec_r8(Regs::D); 1 }
fn dec_25(cpu: &mut Cpu) -> u8 { cpu.dec_r8(Regs::H); 1 }
fn dec_35(cpu: &mut Cpu) -> u8 { cpu.dec_r8(Regs::HL); 3 }

fn dec_0b(cpu: &mut Cpu) -> u8 { cpu.dec_r16(Regs16::BC); cpu.internal_delay(); 2 }
fn dec_1b(cpu: &mut Cpu) -> u8 { cpu.dec_r16(Regs16::DE); cpu.internal_delay(); 2 }
fn dec_2b(cpu: &mut Cpu) -> u8 { cpu.dec_r16(Regs16::HL); cpu.internal_delay(); 2 }
fn dec_3b(cpu: &mut Cpu) -> u8 { cpu.dec_r16(Regs16::SP); cpu.internal_delay(); 2 }

fn dec_0d(cpu: &mut Cpu) -> u8 { cpu.dec_r8(Regs::C); 1 }
fn dec_1d(cpu: &mut Cpu) -> u8 { cpu.dec_r8(Regs::E); 1 }
fn dec_2d(cpu: &mut Cpu) -> u8 { cpu.dec_r8(Regs::L); 1 }
fn dec_3d(cpu: &mut Cpu) -> u8 { cpu.dec_r8(Regs::A); 1 }

fn ld_40(_cpu: &mut Cpu) -> u8 { 1 }
fn ld_50(cpu: &mut Cpu) -> u8 { let val = cpu.get_r8(Regs::B); cpu.set_r8(Regs::D, val); 1 }
fn ld_60(cpu: &mut Cpu) -> u8 { let val = cpu.get_r8(Regs::B); cpu.set_r8(Regs::H, val); 1 }
fn ld_70(cpu: &mut Cpu) -> u8 { let addr = cpu.get_r16(Regs16::HL); let val = cpu.get_r8(Regs::B); cpu.mem_write(addr, val); 2 }

fn ld_41(cpu: &mut Cpu) -> u8 { let val = cpu.get_r8(Regs::C); cpu.set_r8(Regs::B, val); 1 }
fn ld_51(cpu: &mut Cpu) -> u8 { let val = cpu.get_r8(Regs::C); cpu.set_r8(Regs::D, val); 1 }
fn ld_61(cpu: &mut Cpu) -> u8 { let val = cpu.get_r8(Regs::C); cpu.set_r8(Regs::H, val); 1 }
fn ld_71(cpu: &mut Cpu) -> u8 { let addr = cpu.get_r16(Regs16::HL); let val = cpu.get_r8(Regs::C); cpu.mem_write(addr, val); 2 }

fn ld_02(cpu: &mut Cpu) -> u8 { let addr = cpu.get_r16(Regs16::BC); let val = cpu.get_r8(Regs::A); cpu.mem_write(addr, val); 2 }
fn ld_12(cpu: &mut Cpu) -> u8 { let addr = cpu.get_r16(Regs16::DE); let val = cpu.get_r8(Regs::A); cpu.mem_write(addr, val); 2 }
fn ld_22(cpu: &mut Cpu) -> u8 {
    let addr = cpu.get_r16(Regs16::HL);
    let val = cpu.get_r8(Regs::A);
    cpu.mem_write(addr, val);
    cpu.set_r16(Regs16::HL, addr.wrapping_add(1));
    2
}
fn ld_32(cpu: &mut Cpu) -> u8 {
    let addr = cpu.get_r16(Regs16::HL);
    let val = cpu.get_r8(Regs::A);
    cpu.mem_write(addr, val);
    cpu.set_r16(Regs16::HL, addr.wrapping_sub(1));
    2
}
fn ld_42(cpu: &mut Cpu) -> u8 { let val = cpu.get_r8(Regs::D); cpu.set_r8(Regs::B, val); 1 }
fn ld_52(_cpu: &mut Cpu) -> u8 { 1 }
fn ld_62(cpu: &mut Cpu) -> u8 { let val = cpu.get_r8(Regs::D); cpu.set_r8(Regs::H, val); 1 }
fn ld_72(cpu: &mut Cpu) -> u8 { let addr = cpu.get_r16(Regs16::HL); let val = cpu.get_r8(Regs::D); cpu.mem_write(addr, val); 2 }

fn ld_43(cpu: &mut Cpu) -> u8 { let val = cpu.get_r8(Regs::E); cpu.set_r8(Regs::B, val); 1 }
fn ld_53(cpu: &mut Cpu) -> u8 { let val = cpu.get_r8(Regs::E); cpu.set_r8(Regs::D, val); 1 }
fn ld_63(cpu: &mut Cpu) -> u8 { let val = cpu.get_r8(Regs::E); cpu.set_r8(Regs::H, val); 1 }
fn ld_73(cpu: &mut Cpu) -> u8 { let addr = cpu.get_r16(Regs16::HL); let val = cpu.get_r8(Regs::E); cpu.mem_write(addr, val); 2 }

fn ld_44(cpu: &mut Cpu) -> u8 { let val = cpu.get_r8(Regs::H); cpu.set_r8(Regs::B, val); 1 }
fn ld_54(cpu: &mut Cpu) -> u8 { let val = cpu.get_r8(Regs::H); cpu.set_r8(Regs::D, val); 1 }
fn ld_64(_cpu: &mut Cpu) -> u8 { 1 }
fn ld_74(cpu: &mut Cpu) -> u8 { let addr = cpu.get_r16(Regs16::HL); let val = cpu.get_r8(Regs::H); cpu.mem_write(addr, val); 2 }

fn ld_45(cpu: &mut Cpu) -> u8 { let val = cpu.get_r8(Regs::L); cpu.set_r8(Regs::B, val); 1 }
fn ld_55(cpu: &mut Cpu) -> u8 { let val = cpu.get_r8(Regs::L); cpu.set_r8(Regs::D, val); 1 }
fn ld_65(cpu: &mut Cpu) -> u8 { let val = cpu.get_r8(Regs::L); cpu.set_r8(Regs::H, val); 1 }
fn ld_75(cpu: &mut Cpu) -> u8 { let addr = cpu.get_r16(Regs16::HL); let val = cpu.get_r8(Regs::L); cpu.mem_write(addr, val); 2 }

fn ld_06(cpu: &mut Cpu) -> u8 { let val = cpu.fetch(); cpu.set_r8(Regs::B, val); 2 }
fn ld_16(cpu: &mut Cpu) -> u8 { let val = cpu.fetch(); cpu.set_r8(Regs::D, val); 2 }
fn ld_26(cpu: &mut Cpu) -> u8 { let val = cpu.fetch(); cpu.set_r8(Regs::H, val); 2 }
fn ld_36(cpu: &mut Cpu) -> u8 { let addr = cpu.get_r16(Regs16::HL); let val = cpu.fetch(); cpu.mem_write(addr, val); 3 }
fn ld_46(cpu: &mut Cpu) -> u8 { let addr = cpu.get_r16(Regs16::HL); let val = cpu.mem_read(addr); cpu.set_r8(Regs::B, val); 2 }
fn ld_56(cpu: &mut Cpu) -> u8 { let addr = cpu.get_r16(Regs16::HL); let val = cpu.mem_read(addr); cpu.set_r8(Regs::D, val); 2 }
fn ld_66(cpu: &mut Cpu) -> u8 { let addr = cpu.get_r16(Regs16::HL); let val = cpu.mem_read(addr); cpu.set_r8(Regs::H, val); 2 }

fn ld_47(cpu: &mut Cpu) -> u8 { let val = cpu.get_r8(Regs::A); cpu.set_r8(Regs::B, val); 1 }
fn ld_57(cpu: &mut Cpu) -> u8 { let val = cpu.get_r8(Regs::A); cpu.set_r8(Regs::D, val); 1 }
fn ld_67(cpu: &mut Cpu) -> u8 { let val = cpu.get_r8(Regs::A); cpu.set_r8(Regs::H, val); 1 }
fn ld_77(cpu: &mut Cpu) -> u8 { let addr = cpu.get_r16(Regs16::HL); let val = cpu.get_r8(Regs::A); cpu.mem_write(addr, val); 2 }

fn ld_48(cpu: &mut Cpu) -> u8 { let val = cpu.get_r8(Regs::B); cpu.set_r8(Regs::C, val); 1 }
fn ld_58(cpu: &mut Cpu) -> u8 { let val = cpu.get_r8(Regs::B); cpu.set_r8(Regs::E, val); 1 }
fn ld_68(cpu: &mut Cpu) -> u8 { let val = cpu.get_r8(Regs::B); cpu.set_r8(Regs::L, val); 1 }
fn ld_78(cpu: &mut Cpu) -> u8 { let val = cpu.get_r8(Regs::B); cpu.set_r8(Regs::A, val); 1 }

fn ld_49(_cpu: &mut Cpu) -> u8 { 1 }
fn ld_59(cpu: &mut Cpu) -> u8 { let val = cpu.get_r8(Regs::C); cpu.set_r8(Regs::E, val); 1 }
fn ld_69(cpu: &mut Cpu) -> u8 { let val = cpu.get_r8(Regs::C); cpu.set_r8(Regs::L, val); 1 }
fn ld_79(cpu: &mut Cpu) -> u8 { let val = cpu.get_r8(Regs::C); cpu.set_r8(Regs::A, val); 1 }

fn ld_0a(cpu: &mut Cpu) -> u8 { let addr = cpu.get_r16(Regs16::BC); let val = cpu.mem_read(addr); cpu.set_r8(Regs::A, val); 2 }
fn ld_1a(cpu: &mut Cpu) -> u8 { let addr = cpu.get_r16(Regs16::DE); let val = cpu.mem_read(addr); cpu.set_r8(Regs::A, val); 2 }
fn ld_2a(cpu: &mut Cpu) -> u8 {
    let addr = cpu.get_r16(Regs16::HL);
    let val = cpu.mem_read(addr);
    cpu.set_r8(Regs::A, val);
    cpu.set_r16(Regs16::HL, addr.wrapping_add(1));
    2
}
fn ld_3a(cpu: &mut Cpu) -> u8 {
    let addr = cpu.get_r16(Regs16::HL);
    let val = cpu.mem_read(addr);
    cpu.set_r8(Regs::A, val);
    cpu.set_r16(Regs16::HL, addr.wrapping_sub(1));
    2
}
fn ld_4a(cpu: &mut Cpu) -> u8 { let val = cpu.get_r8(Regs::D); cpu.set_r8(Regs::C, val); 1 }
fn ld_5a(cpu: &mut Cpu) -> u8 { let val = cpu.get_r8(Regs::D); cpu.set_r8(Regs::E, val); 1 }
fn ld_6a(cpu: &mut Cpu) -> u8 { let val = cpu.get_r8(Regs::D); cpu.set_r8(Regs::L, val); 1 }
fn ld_7a(cpu: &mut Cpu) -> u8 { let val = cpu.get_r8(Regs::D); cpu.set_r8(Regs::A, val); 1 }

fn ld_4b(cpu: &mut Cpu) -> u8 { let val = cpu.get_r8(Regs::E); cpu.set_r8(Regs::C, val); 1 }
fn ld_5b(_cpu: &mut Cpu) -> u8 { 1 }
fn ld_6b(cpu: &mut Cpu) -> u8 { let val = cpu.get_r8(Regs::E); cpu.set_r8(Regs::L, val); 1 }
fn ld_7b(cpu: &mut Cpu) -> u8 { let val = cpu.get_r8(Regs::E); cpu.set_r8(Regs::A, val); 1 }

fn ld_4c(cpu: &mut Cpu) -> u8 { let val = cpu.get_r8(Regs::H); cpu.set_r8(Regs::C, val); 1 }
fn ld_5c(cpu: &mut Cpu) -> u8 { let val = cpu.get_r8(Regs::H); cpu.set_r8(Regs::E, val); 1 }
fn ld_6c(cpu: &mut Cpu) -> u8 { let val = cpu.get_r8(Regs::H); cpu.set_r8(Regs::L, val); 1 }
fn ld_7c(cpu: &mut Cpu) -> u8 { let val = cpu.get_r8(Regs::H); cpu.set_r8(Regs::A, val); 1 }

fn ld_4d(cpu: &mut Cpu) -> u8 { let val = cpu.get_r8(Regs::L); cpu.set_r8(Regs::C, val); 1 }
fn ld_5d(cpu: &mut Cpu) -> u8 { let val = cpu.get_r8(Regs::L); cpu.set_r8(Regs::E, val); 1 }
fn ld_6d(_cpu: &mut Cpu) -> u8 { 1 }
fn ld_7d(cpu: &mut Cpu) -> u8 { let val = cpu.get_r8(Regs::L); cpu.set_r8(Regs::A, val); 1 }

fn ld_0e(cpu: &mut Cpu) -> u8 { let val = cpu.fetch(); cpu.set_r8(Regs::C, val); 2 }
fn ld_1e(cpu: &mut Cpu) -> u8 { let val = cpu.fetch(); cpu.set_r8(Regs::E, val); 2 }
fn ld_2e(cpu: &mut Cpu) -> u8 { let val = cpu.fetch(); cpu.set_r8(Regs::L, val); 2 }
fn ld_3e(cpu: &mut Cpu) -> u8 { let val = cpu.fetch(); cpu.set_r8(Regs::A, val); 2 }
fn ld_4e(cpu: &mut Cpu) -> u8 { let addr = cpu.get_r16(Regs16::HL); let val = cpu.mem_read(addr); cpu.set_r8(Regs::C, val); 2 }
fn ld_5e(cpu: &mut Cpu) -> u8 { let addr = cpu.get_r16(Regs16::HL); let val = cpu.mem_read(addr); cpu.set_r8(Regs::E, val); 2 }
fn ld_6e(cpu: &mut Cpu) -> u8 { let addr = cpu.get_r16(Regs16::HL); let val = cpu.mem_read(addr); cpu.set_r8(Regs::L, val); 2 }
fn ld_7e(cpu: &mut Cpu) -> u8 { let addr = cpu.get_r16(Regs16::HL); let val = cpu.mem_read(addr); cpu.set_r8(Regs::A, val); 2 }

fn ld_4f(cpu: &mut Cpu) -> u8 { let val = cpu.get_r8(Regs::A); cpu.set_r8(Regs::C, val); 1 }
fn ld_5f(cpu: &mut Cpu) -> u8 { let val = cpu.get_r8(Regs::A); cpu.set_r8(Regs::E, val); 1 }
fn ld_6f(cpu: &mut Cpu) -> u8 { let val = cpu.get_r8(Regs::A); cpu.set_r8(Regs::L, val); 1 }
fn ld_7f(_cpu: &mut Cpu) -> u8 { 1 }

fn ld_08(cpu: &mut Cpu) -> u8 {
    let addr = cpu.fetch_u16();
    let val = cpu.get_r16(Regs16::SP);
    cpu.mem_write(addr, val.low_byte());
    cpu.mem_write(addr.wrapping_add(1), val.high_byte());
    5
}
fn ld_01(cpu: &mut Cpu) -> u8 { let val = cpu.fetch_u16(); cpu.set_r16(Regs16::BC, val); 3 }
fn ld_11(cpu: &mut Cpu) -> u8 { let val = cpu.fetch_u16(); cpu.set_r16(Regs16::DE, val); 3 }
fn ld_21(cpu: &mut Cpu) -> u8 { let val = cpu.fetch_u16(); cpu.set_r16(Regs16::HL, val); 3 }
fn ld_31(cpu: &mut Cpu) -> u8 { let val = cpu.fetch_u16(); cpu.set_r16(Regs16::SP, val); 3 }

fn ld_f9(cpu: &mut Cpu) -> u8 {
    let val = cpu.get_r16(Regs16::HL);
    cpu.set_r16(Regs16::SP, val);
    cpu.internal_delay();
    2
}
fn ld_e0(cpu: &mut Cpu) -> u8 { let offset = cpu.fetch() as u16; let val = cpu.get_r8(Regs::A); cpu.mem_write(0xFF00 + offset, val); 2 }
fn ld_f0(cpu: &mut Cpu) -> u8 { let offset = cpu.fetch() as u16; let val = cpu.mem_read(0xFF00 + offset); cpu.set_r8(Regs::A, val); 2 }
fn ld_e2(cpu: &mut Cpu) -> u8 { let offset = cpu.get_r8(Regs::C) as u16; let val = cpu.get_r8(Regs::A); cpu.mem_write(0xFF00 + offset, val); 2 }
fn ld_f2(cpu: &mut Cpu) -> u8 { let offset = cpu.get_r8(Regs::C) as u16; let val = cpu.mem_read(0xFF00 + offset); cpu.set_r8(Regs::A, val); 2 }
fn ld_ea(cpu: &mut Cpu) -> u8 { let addr = cpu.fetch_u16(); let val = cpu.get_r8(Regs::A); cpu.mem_write(addr, val); 4 }
fn ld_fa(cpu: &mut Cpu) -> u8 { let addr = cpu.fetch_u16(); let val = cpu.mem_read(addr); cpu.set_r8(Regs::A, val); 4 }

fn ld_f8(cpu: &mut Cpu) -> u8 {
    let offset = cpu.fetch() as i8 as i16 as u16;
    let sp = cpu.get_r16(Regs16::SP);
    let set_c = check_c_carry_u8(sp.low_byte(), offset.low_byte());
    let set_h = check_h_carry_u8(sp.low_byte(), offset.low_byte());

    cpu.internal_delay();
    cpu.set_r16(Regs16::HL, sp.wrapping_add(offset));
    cpu.set_flag(Flags::Z, false);
    cpu.set_flag(Flags::N, false);
    cpu.set_flag(Flags::C, set_c);
    cpu.set_flag(Flags::H, set_h);
    3
}

fn add_09(cpu: &mut Cpu) -> u8 { cpu.add_r16(Regs16::HL, Regs16::BC); cpu.internal_delay(); 2 }
fn add_19(cpu: &mut Cpu) -> u8 { cpu.add_r16(Regs16::HL, Regs16::DE); cpu.internal_delay(); 2 }
fn add_29(cpu: &mut Cpu) -> u8 { cpu.add_r16(Regs16::HL, Regs16::HL); cpu.internal_delay(); 2 }
fn add_39(cpu: &mut Cpu) -> u8 { cpu.add_r16(Regs16::HL, Regs16::SP); cpu.internal_delay(); 2 }

fn add_80(cpu: &mut Cpu) -> u8 { let val = cpu.get_r8(Regs::B); cpu.add_a_u8(val, false); 1 }
fn add_81(cpu: &mut Cpu) -> u8 { let val = cpu.get_r8(Regs::C); cpu.add_a_u8(val, false); 1 }
fn add_82(cpu: &mut Cpu) -> u8 { let val = cpu.get_r8(Regs::D); cpu.add_a_u8(val, false); 1 }
fn add_83(cpu: &mut Cpu) -> u8 { let val = cpu.get_r8(Regs::E); cpu.add_a_u8(val, false); 1 }
fn add_84(cpu: &mut Cpu) -> u8 { let val = cpu.get_r8(Regs::H); cpu.add_a_u8(val, false); 1 }
fn add_85(cpu: &mut Cpu) -> u8 { let val = cpu.get_r8(Regs::L); cpu.add_a_u8(val, false); 1 }
fn add_86(cpu: &mut Cpu) -> u8 { let val = cpu.get_r8(Regs::HL); cpu.add_a_u8(val, false); 2 }
fn add_87(cpu: &mut Cpu) -> u8 { let val = cpu.get_r8(Regs::A); cpu.add_a_u8(val, false); 1 }

fn add_88(cpu: &mut Cpu) -> u8 { let val = cpu.get_r8(Regs::B); cpu.add_a_u8(val, true); 1 }
fn add_89(cpu: &mut Cpu) -> u8 { let val = cpu.get_r8(Regs::C); cpu.add_a_u8(val, true); 1 }
fn add_8a(cpu: &mut Cpu) -> u8 { let val = cpu.get_r8(Regs::D); cpu.add_a_u8(val, true); 1 }
fn add_8b(cpu: &mut Cpu) -> u8 { let val = cpu.get_r8(Regs::E); cpu.add_a_u8(val, true); 1 }
fn add_8c(cpu: &mut Cpu) -> u8 { let val = cpu.get_r8(Regs::H); cpu.add_a_u8(val, true); 1 }
fn add_8d(cpu: &mut Cpu) -> u8 { let val = cpu.get_r8(Regs::L); cpu.add_a_u8(val, true); 1 }
fn add_8e(cpu: &mut Cpu) -> u8 { let val = cpu.get_r8(Regs::HL); cpu.add_a_u8(val, true); 2 }
fn add_8f(cpu: &mut Cpu) -> u8 { let val = cpu.get_r8(Regs::A); cpu.add_a_u8(val, true); 1 }

fn sub_90(cpu: &mut Cpu) -> u8 { let val = cpu.get_r8(Regs::B); cpu.sub_a_u8(val, false); 1 }
fn sub_91(cpu: &mut Cpu) -> u8 { let val = cpu.get_r8(Regs::C); cpu.sub_a_u8(val, false); 1 }
fn sub_92(cpu: &mut Cpu) -> u8 { let val = cpu.get_r8(Regs::D); cpu.sub_a_u8(val, false); 1 }
fn sub_93(cpu: &mut Cpu) -> u8 { let val = cpu.get_r8(Regs::E); cpu.sub_a_u8(val, false); 1 }
fn sub_94(cpu: &mut Cpu) -> u8 { let val = cpu.get_r8(Regs::H); cpu.sub_a_u8(val, false); 1 }
fn sub_95(cpu: &mut Cpu) -> u8 { let val = cpu.get_r8(Regs::L); cpu.sub_a_u8(val, false); 1 }
fn sub_96(cpu: &mut Cpu) -> u8 { let val = cpu.get_r8(Regs::HL); cpu.sub_a_u8(val, false); 2 }
fn sub_97(cpu: &mut Cpu) -> u8 { let val = cpu.get_r8(Regs::A); cpu.sub_a_u8(val, false); 1 }

fn sbc_98(cpu: &mut Cpu) -> u8 { let val = cpu.get_r8(Regs::B); cpu.sub_a_u8(val, true); 1 }
fn sbc_99(cpu: &mut Cpu) -> u8 { let val = cpu.get_r8(Regs::C); cpu.sub_a_u8(val, true); 1 }
fn sbc_9a(cpu: &mut Cpu) -> u8 { let val = cpu.get_r8(Regs::D); cpu.sub_a_u8(val, true); 1 }
fn sbc_9b(cpu: &mut Cpu) -> u8 { let val = cpu.get_r8(Regs::E); cpu.sub_a_u8(val, true); 1 }
fn sbc_9c(cpu: &mut Cpu) -> u8 { let val = cpu.get_r8(Regs::H); cpu.sub_a_u8(val, true); 1 }
fn sbc_9d(cpu: &mut Cpu) -> u8 { let val = cpu.get_r8(Regs::L); cpu.sub_a_u8(val, true); 1 }
fn sbc_9e(cpu: &mut Cpu) -> u8 { let val = cpu.get_r8(Regs::HL); cpu.sub_a_u8(val, true); 2 }
fn sbc_9f(cpu: &mut Cpu) -> u8 { let val = cpu.get_r8(Regs::A); cpu.sub_a_u8(val, true); 1 }

fn and_a0(cpu: &mut Cpu) -> u8 { let val = cpu.get_r8(Regs::B); cpu.and_a_u8(val); 1 }
fn and_a1(cpu: &mut Cpu) -> u8 { let val = cpu.get_r8(Regs::C); cpu.and_a_u8(val); 1 }
fn and_a2(cpu: &mut Cpu) -> u8 { let val = cpu.get_r8(Regs::D); cpu.and_a_u8(val); 1 }
fn and_a3(cpu: &mut Cpu) -> u8 { let val = cpu.get_r8(Regs::E); cpu.and_a_u8(val); 1 }
fn and_a4(cpu: &mut Cpu) -> u8 { let val = cpu.get_r8(Regs::H); cpu.and_a_u8(val); 1 }
fn and_a5(cpu: &mut Cpu) -> u8 { let val = cpu.get_r8(Regs::L); cpu.and_a_u8(val); 1 }
fn and_a6(cpu: &mut Cpu) -> u8 { let val = cpu.get_r8(Regs::HL); cpu.and_a_u8(val); 2 }
fn and_a7(cpu: &mut Cpu) -> u8 { let val = cpu.get_r8(Regs::A); cpu.and_a_u8(val); 1 }

fn xor_a8(cpu: &mut Cpu) -> u8 { let val = cpu.get_r8(Regs::B); cpu.xor_a_u8(val); 1 }
fn xor_a9(cpu: &mut Cpu) -> u8 { let val = cpu.get_r8(Regs::C); cpu.xor_a_u8(val); 1 }
fn xor_aa(cpu: &mut Cpu) -> u8 { let val = cpu.get_r8(Regs::D); cpu.xor_a_u8(val); 1 }
fn xor_ab(cpu: &mut Cpu) -> u8 { let val = cpu.get_r8(Regs::E); cpu.xor_a_u8(val); 1 }
fn xor_ac(cpu: &mut Cpu) -> u8 { let val = cpu.get_r8(Regs::H); cpu.xor_a_u8(val); 1 }
fn xor_ad(cpu: &mut Cpu) -> u8 { let val = cpu.get_r8(Regs::L); cpu.xor_a_u8(val); 1 }
fn xor_ae(cpu: &mut Cpu) -> u8 { let val = cpu.get_r8(Regs::HL); cpu.xor_a_u8(val); 2 }
fn xor_af(cpu: &mut Cpu) -> u8 { let val = cpu.get_r8(Regs::A); cpu.xor_a_u8(val); 1 }

fn or_b0(cpu: &mut Cpu) -> u8 { let val = cpu.get_r8(Regs::B); cpu.or_a_u8(val); 1 }
fn or_b1(cpu: &mut Cpu) -> u8 { let val = cpu.get_r8(Regs::C); cpu.or_a_u8(val); 1 }
fn or_b2(cpu: &mut Cpu) -> u8 { let val = cpu.get_r8(Regs::D); cpu.or_a_u8(val); 1 }
fn or_b3(cpu: &mut Cpu) -> u8 { let val = cpu.get_r8(Regs::E); cpu.or_a_u8(val); 1 }
fn or_b4(cpu: &mut Cpu) -> u8 { let val = cpu.get_r8(Regs::H); cpu.or_a_u8(val); 1 }
fn or_b5(cpu: &mut Cpu) -> u8 { let val = cpu.get_r8(Regs::L); cpu.or_a_u8(val); 1 }
fn or_b6(cpu: &mut Cpu) -> u8 { let val = cpu.get_r8(Regs::HL); cpu.or_a_u8(val); 2 }
fn or_b7(cpu: &mut Cpu) -> u8 { let val = cpu.get_r8(Regs::A); cpu.or_a_u8(val); 1 }

fn cp_b8(cpu: &mut Cpu) -> u8 { let val = cpu.get_r8(Regs::B); cpu.cp_a_u8(val); 1 }
fn cp_b9(cpu: &mut Cpu) -> u8 { let val = cpu.get_r8(Regs::C); cpu.cp_a_u8(val); 1 }
fn cp_ba(cpu: &mut Cpu) -> u8 { let val = cpu.get_r8(Regs::D); cpu.cp_a_u8(val); 1 }
fn cp_bb(cpu: &mut Cpu) -> u8 { let val = cpu.get_r8(Regs::E); cpu.cp_a_u8(val); 1 }
fn cp_bc(cpu: &mut Cpu) -> u8 { let val = cpu.get_r8(Regs::H); cpu.cp_a_u8(val); 1 }
fn cp_bd(cpu: &mut Cpu) -> u8 { let val = cpu.get_r8(Regs::L); cpu.cp_a_u8(val); 1 }
fn cp_be(cpu: &mut Cpu) -> u8 { let val = cpu.get_r8(Regs::HL); cpu.cp_a_u8(val); 2 }
fn cp_bf(cpu: &mut Cpu) -> u8 { let val = cpu.get_r8(Regs::A); cpu.cp_a_u8(val); 1 }

fn add_c6(cpu: &mut Cpu) -> u8 { let val = cpu.fetch(); cpu.add_a_u8(val, false); 2 }
fn adc_ce(cpu: &mut Cpu) -> u8 { let val = cpu.fetch(); cpu.add_a_u8(val, true); 2 }
fn sub_d6(cpu: &mut Cpu) -> u8 { let val = cpu.fetch(); cpu.sub_a_u8(val, false); 2 }
fn sbc_de(cpu: &mut Cpu) -> u8 { let val = cpu.fetch(); cpu.sub_a_u8(val, true); 2 }
fn and_e6(cpu: &mut Cpu) -> u8 { let val = cpu.fetch(); cpu.and_a_u8(val); 2 }
fn xor_ee(cpu: &mut Cpu) -> u8 { let val = cpu.fetch(); cpu.xor_a_u8(val); 2 }
fn or_f6(cpu: &mut Cpu) -> u8 { let val = cpu.fetch(); cpu.or_a_u8(val); 2 }
fn cp_fe(cpu: &mut Cpu) -> u8 { let val = cpu.fetch(); cpu.cp_a_u8(val); 2 }

fn add_e8(cpu: &mut Cpu) -> u8 {
    let offset = cpu.fetch() as i8 as i16 as u16;
    let sp = cpu.get_r16(Regs16::SP);
    let set_c = check_c_carry_u8(sp.low_byte(), offset.low_byte());
    let set_h = check_h_carry_u8(sp.low_byte(), offset.low_byte());

    cpu.internal_delay();
    cpu.internal_delay();
    cpu.set_r16(Regs16::SP, sp.wrapping_add(offset));
    cpu.set_flag(Flags::Z, false);
    cpu.set_flag(Flags::N, false);
    cpu.set_flag(Flags::H, set_h);
    cpu.set_flag(Flags::C, set_c);
    4
}

fn pop_c1(cpu: &mut Cpu) -> u8 { let val = cpu.pop(); cpu.set_r16(Regs16::BC, val); 3 }
fn pop_d1(cpu: &mut Cpu) -> u8 { let val = cpu.pop(); cpu.set_r16(Regs16::DE, val); 3 }
fn pop_e1(cpu: &mut Cpu) -> u8 { let val = cpu.pop(); cpu.set_r16(Regs16::HL, val); 3 }
fn pop_f1(cpu: &mut Cpu) -> u8 { let val = cpu.pop(); cpu.set_r16(Regs16::AF, val); 3 }

fn push_c5(cpu: &mut Cpu) -> u8 { let val = cpu.get_r16(Regs16::BC); cpu.push(val); 4 }
fn push_d5(cpu: &mut Cpu) -> u8 { let val = cpu.get_r16(Regs16::DE); cpu.push(val); 4 }
fn push_e5(cpu: &mut Cpu) -> u8 { let val = cpu.get_r16(Regs16::HL); cpu.push(val); 4 }
fn push_f5(cpu: &mut Cpu) -> u8 { let val = cpu.get_r16(Regs16::AF); cpu.push(val); 4 }

fn jr_18(cpu: &mut Cpu) -> u8 {
    let offset = cpu.fetch() as i8 as i16 as u16;
    cpu.internal_delay();
    let pc = cpu.get_pc().wrapping_add(offset);
    cpu.set_pc(pc);
    3
}

fn jp_c3(cpu: &mut Cpu) -> u8 {
    let addr = cpu.fetch_u16();
    cpu.internal_delay();
    cpu.set_pc(addr);
    4
}
fn jp_e9(cpu: &mut Cpu) -> u8 { let addr = cpu.get_r16(Regs16::HL); cpu.set_pc(addr); 1 }

fn jp_c2(cpu: &mut Cpu) -> u8 {
    let addr = cpu.fetch_u16();
    if !cpu.get_flag(Flags::Z) {
        cpu.internal_delay();
        cpu.set_pc(addr);
        4
    } else {
        3
    }
}
fn jp_ca(cpu: &mut Cpu) -> u8 {
    let addr = cpu.fetch_u16();
    if cpu.get_flag(Flags::Z) {
        cpu.internal_delay();
        cpu.set_pc(addr);
        4
    } else {
        3
    }
}
fn jp_d2(cpu: &mut Cpu) -> u8 {
    let addr = cpu.fetch_u16();
    if !cpu.get_flag(Flags::C) {
        cpu.internal_delay();
        cpu.set_pc(addr);
        4
    } else {
        3
    }
}
fn jp_da(cpu: &mut Cpu) -> u8 {
    let addr = cpu.fetch_u16();
    if cpu.get_flag(Flags::C) {
        cpu.internal_delay();
        cpu.set_pc(addr);
        4
    } else {
        3
    }
}

fn jr_20(cpu: &mut Cpu) -> u8 {
    let offset = cpu.fetch() as i8 as i16 as u16;
    if !cpu.get_flag(Flags::Z) {
        cpu.internal_delay();
        cpu.set_pc(cpu.get_pc().wrapping_add(offset));
        3
    } else {
        2
    }
}
fn jr_28(cpu: &mut Cpu) -> u8 {
    let offset = cpu.fetch() as i8 as i16 as u16;
    if cpu.get_flag(Flags::Z) {
        cpu.internal_delay();
        cpu.set_pc(cpu.get_pc().wrapping_add(offset));
        3
    } else {
        2
    }
}
fn jr_30(cpu: &mut Cpu) -> u8 {
    let offset = cpu.fetch() as i8 as i16 as u16;
    if !cpu.get_flag(Flags::C) {
        cpu.internal_delay();
        cpu.set_pc(cpu.get_pc().wrapping_add(offset));
        3
    } else {
        2
    }
}
fn jr_38(cpu: &mut Cpu) -> u8 {
    let offset = cpu.fetch() as i8 as i16 as u16;
    if cpu.get_flag(Flags::C) {
        cpu.internal_delay();
        cpu.set_pc(cpu.get_pc().wrapping_add(offset));
        3
    } else {
        2
    }
}

fn call_cd(cpu: &mut Cpu) -> u8 {
    let addr = cpu.fetch_u16();
    cpu.push(cpu.get_pc());
    cpu.set_pc(addr);
    6
}
fn call_cc(cpu: &mut Cpu) -> u8 {
    let addr = cpu.fetch_u16();
    if cpu.get_flag(Flags::Z) {
        cpu.push(cpu.get_pc());
        cpu.set_pc(addr);
        6
    } else {
        3
    }
}
fn call_dc(cpu: &mut Cpu) -> u8 {
    let addr = cpu.fetch_u16();
    if cpu.get_flag(Flags::C) {
        cpu.push(cpu.get_pc());
        cpu.set_pc(addr);
        6
    } else {
        3
    }
}
fn call_c4(cpu: &mut Cpu) -> u8 {
    let addr = cpu.fetch_u16();
    if !cpu.get_flag(Flags::Z) {
        cpu.push(cpu.get_pc());
        cpu.set_pc(addr);
        6
    } else {
        3
    }
}
fn call_d4(cpu: &mut Cpu) -> u8 {
    let addr = cpu.fetch_u16();
    if !cpu.get_flag(Flags::C) {
        cpu.push(cpu.get_pc());
        cpu.set_pc(addr);
        6
    } else {
        3
    }
}

fn ret_c9(cpu: &mut Cpu) -> u8 {
    let addr = cpu.pop();
    cpu.internal_delay();
    cpu.set_pc(addr);
    4
}
fn ret_c8(cpu: &mut Cpu) -> u8 {
    cpu.internal_delay();
    if cpu.get_flag(Flags::Z) {
        let addr = cpu.pop();
        cpu.internal_delay();
        cpu.set_pc(addr);
        5
    } else {
        2
    }
}
fn ret_d8(cpu: &mut Cpu) -> u8 {
    cpu.internal_delay();
    if cpu.get_flag(Flags::C) {
        let addr = cpu.pop();
        cpu.internal_delay();
        cpu.set_pc(addr);
        5
    } else {
        2
    }
}
fn ret_c0(cpu: &mut Cpu) -> u8 {
    cpu.internal_delay();
    if !cpu.get_flag(Flags::Z) {
        let addr = cpu.pop();
        cpu.internal_delay();
        cpu.set_pc(addr);
        5
    } else {
        2
    }
}
fn ret_d0(cpu: &mut Cpu) -> u8 {
    cpu.internal_delay();
    if !cpu.get_flag(Flags::C) {
        let addr = cpu.pop();
        cpu.internal_delay();
        cpu.set_pc(addr);
        5
    } else {
        2
    }
}

fn rst_c7(cpu: &mut Cpu) -> u8 { cpu.push(cpu.get_pc()); cpu.set_pc(0x0000); 4 }
fn rst_cf(cpu: &mut Cpu) -> u8 { cpu.push(cpu.get_pc()); cpu.set_pc(0x0008); 4 }
fn rst_d7(cpu: &mut Cpu) -> u8 { cpu.push(cpu.get_pc()); cpu.set_pc(0x0010); 4 }
fn rst_df(cpu: &mut Cpu) -> u8 { cpu.push(cpu.get_pc()); cpu.set_pc(0x0018); 4 }
fn rst_e7(cpu: &mut Cpu) -> u8 { cpu.push(cpu.get_pc()); cpu.set_pc(0x0020); 4 }
fn rst_ef(cpu: &mut Cpu) -> u8 { cpu.push(cpu.get_pc()); cpu.set_pc(0x0028); 4 }
fn rst_f7(cpu: &mut Cpu) -> u8 { cpu.push(cpu.get_pc()); cpu.set_pc(0x0030); 4 }
fn rst_ff(cpu: &mut Cpu) -> u8 { cpu.push(cpu.get_pc()); cpu.set_pc(0x0038); 4 }

fn rlca_07(cpu: &mut Cpu) -> u8 { cpu.rotate_left(Regs::A, false); cpu.set_flag(Flags::Z, false); 1 }
fn rrca_0f(cpu: &mut Cpu) -> u8 { cpu.rotate_right(Regs::A, false); cpu.set_flag(Flags::Z, false); 1 }
fn rla_17(cpu: &mut Cpu) -> u8 { cpu.rotate_left(Regs::A, true); cpu.set_flag(Flags::Z, false); 1 }
fn rra_1f(cpu: &mut Cpu) -> u8 { cpu.rotate_right(Regs::A, true); cpu.set_flag(Flags::Z, false); 1 }

fn scf_37(cpu: &mut Cpu) -> u8 {
    cpu.set_flag(Flags::N, false);
    cpu.set_flag(Flags::H, false);
    cpu.set_flag(Flags::C, true);
    1
}

fn ccf_3f(cpu: &mut Cpu) -> u8 {
    let c = cpu.get_flag(Flags::C);
    cpu.set_flag(Flags::N, false);
    cpu.set_flag(Flags::H, false);
    cpu.set_flag(Flags::C, !c);
    1
}

fn cpl_2f(cpu: &mut Cpu) -> u8 {
    let a = cpu.get_r8(Regs::A);
    cpu.set_r8(Regs::A, !a);
    cpu.set_flag(Flags::N, true);
    cpu.set_flag(Flags::H, true);
    1
}

fn reti_d9(cpu: &mut Cpu) -> u8 {
    let addr = cpu.pop();
    cpu.internal_delay();
    cpu.set_pc(addr);
    cpu.irq_enabled = true;
    4
}

fn di_f3(cpu: &mut Cpu) -> u8 { cpu.irq_enabled = false; 1 }
fn ei_fb(cpu: &mut Cpu) -> u8 { cpu.ime_scheduled = true; 1 }
fn stop_10(_cpu: &mut Cpu) -> u8 { 1 }
fn halt_76(cpu: &mut Cpu) -> u8 { cpu.set_halted(true); 1 }
fn invalid(_cpu: &mut Cpu) -> u8 { panic!("Invalid opcode"); }

fn daa_27(cpu: &mut Cpu) -> u8 {
    let mut a = cpu.get_r8(Regs::A) as u16;

    if cpu.get_flag(Flags::N) {
        if cpu.get_flag(Flags::H) {
            a = a.wrapping_sub(0x06) & 0xFF;
        }
        if cpu.get_flag(Flags::C) {
            a = a.wrapping_sub(0x60);
        }
    } else {
        if cpu.get_flag(Flags::H) || (a & 0x0F) > 0x09 {
            a += 0x06;
        }
        if cpu.get_flag(Flags::C) || a > 0x9F {
            a += 0x60;
            cpu.set_flag(Flags::C, true);
        }
    }

    let result = a as u8;
    cpu.set_r8(Regs::A, result);
    cpu.set_flag(Flags::Z, result == 0);
    cpu.set_flag(Flags::H, false);
    1
}