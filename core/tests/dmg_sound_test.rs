use gb_core::cpu::Cpu;
use std::fs;

fn run_single_test(test_name: &str) -> (u8, String) {
    let path = format!("../test/dmg_sound/rom_singles/{}", test_name);
    let mut gb = Cpu::new();
    let rom = match fs::read(&path) {
        Ok(r) => r,
        Err(_) => return (0xFE, format!("Could not read {}", path)),
    };
    gb.load_rom(&rom);

    let mut started = false;

    for _ in 0..15_000_000 {
        gb.tick();

        let sig1 = gb.read_ram(0xA001);
        let sig2 = gb.read_ram(0xA002);
        let sig3 = gb.read_ram(0xA003);

        if sig1 == 0xDE && sig2 == 0xB0 && sig3 == 0x61 {
            let res = gb.read_ram(0xA000);
            if res == 0x80 {
                started = true;
            } else if started {
                let mut s = String::new();
                let mut addr = 0xA004;
                while addr < 0xC000 {
                    let b = gb.read_ram(addr);
                    if b == 0 {
                        break;
                    }
                    s.push(b as char);
                    addr += 1;
                }
                return (res, s);
            }
        }
    }
    (0xFF, "TIMEOUT".to_string())
}

#[test]
fn test_all_dmg_sound() {
    let tests = [
        "01-registers.gb",
        "02-len ctr.gb",
        "03-trigger.gb",
        "04-sweep.gb",
        "05-sweep details.gb",
        "06-overflow on trigger.gb",
        "07-len sweep period sync.gb",
        "08-len ctr during power.gb",
        "09-wave read while on.gb",
        "10-wave trigger while on.gb",
        "11-regs after power.gb",
        "12-wave write while on.gb",
    ];

    let mut all_passed = true;

    for t in tests {
        let (res, out) = run_single_test(t);
        println!("=== {} ===", t);
        println!("Result Code: {}", res);
        println!("Output:\n{}", out);
        if res != 0 {
            all_passed = false;
        }
    }

    assert!(all_passed, "Some dmg_sound tests failed!");
}
