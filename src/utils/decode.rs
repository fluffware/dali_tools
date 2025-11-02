const CMD_DESCR_16: [&str; 256] = [
    "Off",
    "Up",
    "Down",
    "Step up",
    "Step down",
    "Recall max level",
    "Recall min level",
    "Step down and off",
    "On and step up",
    "Enable DPAC sequence",
    "Go to last active level", // 0x0a
    "",
    "",
    "",
    "",
    "",
    "Go to scene 0", // 0x10
    "Go to scene 1",
    "Go to scene 2",
    "Go to scene 3",
    "Go to scene 4",
    "Go to scene 5",
    "Go to scene 6",
    "Go to scene 7",
    "Go to scene 8",
    "Go to scene 9",
    "Go to scene 10",
    "Go to scene 11",
    "Go to scene 12",
    "Go to scene 13",
    "Go to scene 14",
    "Go to scene 15",
    "Reset", // 0x20
    "Store actual level in DTR",
    "Save persistent variables",
    "Set operating mode",
    "Reset memory bank",
    "Identify device",
    "",
    "",
    "",
    "",
    "Store DTR as max level", // 0X2A
    "Store DTR as min level",
    "Store DTR as system failure level",
    "Store DTR as power on level",
    "Store DTR as fade time",
    "Store DTR as fade rate",
    "Set extended fade time", // 0x30
    "",
    "",
    "",
    "",
    "",
    "",
    "",
    "",
    "",
    "",
    "",
    "",
    "",
    "",
    "",
    "Set scene 0", // 0x40
    "Set scene 1",
    "Set scene 2",
    "Set scene 3",
    "Set scene 4",
    "Set scene 5",
    "Set scene 6",
    "Set scene 7",
    "Set scene 8",
    "Set scene 9",
    "Set scene 10",
    "Set scene 11",
    "Set scene 12",
    "Set scene 13",
    "Set scene 14",
    "Set scene 15",
    "Remove from scene 0", // 0x50
    "Remove from scene 1",
    "Remove from scene 2",
    "Remove from scene 3",
    "Remove from scene 4",
    "Remove from scene 5",
    "Remove from scene 6",
    "Remove from scene 7",
    "Remove from scene 8",
    "Remove from scene 9",
    "Remove from scene 10",
    "Remove from scene 11",
    "Remove from scene 12",
    "Remove from scene 13",
    "Remove from scene 14",
    "Remove from scene 15",
    "Add to group 0", // 0x60
    "Add to group 1",
    "Add to group 2",
    "Add to group 3",
    "Add to group 4",
    "Add to group 5",
    "Add to group 6",
    "Add to group 7",
    "Add to group 8",
    "Add to group 9",
    "Add to group 10",
    "Add to group 11",
    "Add to group 12",
    "Add to group 13",
    "Add to group 14",
    "Add to group 15",
    "Remove from group 0", // 0x70
    "Remove from group 1",
    "Remove from group 2",
    "Remove from group 3",
    "Remove from group 4",
    "Remove from group 5",
    "Remove from group 6",
    "Remove from group 7",
    "Remove from group 8",
    "Remove from group 9",
    "Remove from group 10",
    "Remove from group 11",
    "Remove from group 12",
    "Remove from group 13",
    "Remove from group 14",
    "Remove from group 15",
    "Store DTR as short address", // 0x80
    "Enable write memory",
    "",
    "",
    "",
    "",
    "",
    "",
    "",
    "",
    "",
    "",
    "",
    "",
    "",
    "",
    "Query status", // 0x90
    "Query ballast",
    "Query lamp failure",
    "Query lamp power on",
    "Query limit error",
    "Query reset state",
    "Query missing short address",
    "Query version number",
    "Query content of DTR",
    "Query device type",
    "Query physical minimum level",
    "Query power failure",
    "Query content of DTR1",
    "Query content of DTR2",
    "Query operating mode",
    "Query light source type",
    "Query actual level", // 0xa0
    "Query max level",
    "Query min level",
    "Query power on level",
    "Query system failure level",
    "Query fade time/fade rate",
    "Query manufacturer specific mode",
    "Query next device type",
    "Query extended fade time",
    "",
    "Query control gear failure", // 0xaa
    "",
    "",
    "",
    "",
    "",
    "Query scene level 0", // 0xb0
    "Query scene level 1",
    "Query scene level 2",
    "Query scene level 3",
    "Query scene level 4",
    "Query scene level 5",
    "Query scene level 6",
    "Query scene level 7",
    "Query scene level 8",
    "Query scene level 9",
    "Query scene level 10",
    "Query scene level 11",
    "Query scene level 12",
    "Query scene level 13",
    "Query scene level 14",
    "Query scene level 15",
    "Query groups 0 - 7", // 0xc0
    "Query groups 8-15",
    "Query random address (H)",
    "Query random address (M)",
    "Query random address (L)",
    "Query memory location", //0xc5
    "",
    "",
    "",
    "",
    "",
    "",
    "",
    "",
    "",
    "",
    "", // 0xd0
    "",
    "",
    "",
    "",
    "",
    "",
    "",
    "",
    "",
    "",
    "",
    "",
    "",
    "",
    "",
    "Application extended command 0xe0",
    "Application extended command 0xe1",
    "Application extended command 0xe2",
    "Application extended command 0xe3",
    "Application extended command 0xe4",
    "Application extended command 0xe5",
    "Application extended command 0xe6",
    "Application extended command 0xe7",
    "Application extended command 0xe8",
    "Application extended command 0xe9",
    "Application extended command 0xea",
    "Application extended command 0xeb",
    "Application extended command 0xec",
    "Application extended command 0xed",
    "Application extended command 0xee",
    "Application extended command 0xef",
    "Application extended command 0xf0",
    "Application extended command 0xf1",
    "Application extended command 0xf2",
    "Application extended command 0xf3",
    "Application extended command 0xf4",
    "Application extended command 0xf5",
    "Application extended command 0xf6",
    "Application extended command 0xf7",
    "Application extended command 0xf8",
    "Application extended command 0xf9",
    "Application extended command 0xfa",
    "Application extended command 0xfb",
    "Application extended command 0xfc",
    "Application extended command 0xfd",
    "Application extended command 0xfe",
    "Query extended verion number", // 0xff
];

fn device_cmd_descr_24(cmd: u8) -> &'static str {
    match cmd {
        0x00 => "Identify device",
        0x01 => "Reset power cycle seen",
        0x10 => "Reset",
        0x11 => "Reset memory bank",
        0x14 => "Set short address",
        0x15 => "Enable write memory",
        0x16 => "Enable application controller",
        0x17 => "Disble application controller",
        0x18 => "Set operating mode",
        0x19 => "Add to device groups 0 - 15",
        0x1a => "Add to device groups 16 - 31",
        0x1b => "Remove from device groups 0 - 15",
        0x1c => "Remove from device groups 16 - 31",
        0x1d => "Start quiescent mode",
        0x1e => "Stop quiescent mode",
        0x1f => "Enable power cycle notification",
        0x20 => "Disble power cycle notification",
        0x21 => "Save persistent variables",
        0x30 => "Query device status",
        0x31 => "Query application controller error",
        0x32 => "Query input device error",
        0x33 => "Query missing short address",
        0x34 => "Query version number",
        0x35 => "Query number of instances",
        0x36 => "Query content DTR0",
        0x37 => "Query content DTR1",
        0x38 => "Query content DTR2",
        0x39 => "Query random address (H)",
        0x3a => "Query random address (M)",
        0x3b => "Query random address (L)",
        0x3c => "Read memory location",
        0x3d => "Query application controller enabled",
        0x3e => "Query operating mode",
        0x3f => "Query manufacturer specific mode",
        0x40 => "Query quiescent mode",
        0x41 => "Query device groups 0 - 7",
        0x42 => "Query device groups 8 - 15",
        0x43 => "Query device groups 16 - 23",
        0x44 => "Query device groups 24 - 31",
        0x45 => "Query power cycle notification",
        0x46 => "Query device capabilities",
        0x47 => "Query extended version number",
        0x48 => "Query reset state",
        _ => "Unknown device command",
    }
}

fn instance_cmd_descr_24(cmd: u8) -> &'static str {
    match cmd {
        0x61 => "Set event priority",
        0x62 => "Enable instance",
        0x63 => "Disable instance",
        0x64 => "Set primary instance group",
        0x65 => "Set instance group 1",
        0x66 => "Set instance group 2",
        0x67 => "Set event scheme",
        0x68 => "Set event filter",
        0x80 => "Query instance type",
        0x81 => "Query resolution",
        0x82 => "Query instance error",
        0x83 => "Query instance status",
        0x84 => "Query event priority",
        0x86 => "Query instance enabled",
        0x88 => "Query primary instance group",
        0x89 => "Query instance group 1",
        0x8a => "Query instance group 2",
        0x8b => "Query event scheme",
        0x8c => "Query input value",
        0x8d => "Query input value latch",
        0x8e => "Query feature type",
        0x8f => "Query next feature type",
        0x90 => "Query event filter 0 - 7",
        0x91 => "Query event filter 8 - 15",
        0x92 => "Query event filter 16 - 23",
        _ => "Unknown instance command",
    }
}

fn decode_addr(addr: u8) -> String {
    if addr & 0xfe == 0xfe {
        // Broadcast
        "Broadcast".to_string()
    } else if addr & 0xfe == 0xfc {
        // Broadcast unaddressed
        "Unaddressed".to_string()
    } else if addr & 0x80 != 0 {
        // Group
        if addr & 0x60 != 0 {
            "Illegal group address".to_string()
        } else {
            format!("Group: {}", (addr >> 1) & 0x0f)
        }
    } else {
        /* Address */
        format!("Addr: {}", (addr >> 1) & 0x3f)
    }
}

fn decode_16bit(pkt: &[u8]) -> String {
    let str;
    assert!(pkt.len() >= 2);
    if (pkt[0] & 1) != 0 {
        // Command
        if (pkt[0] & 0xe0) == 0xa0 || (pkt[0] & 0xe0) == 0xc0 {
            // Special command
            str = match pkt[0] {
                0xa1 => "Terminate".to_string(),
                0xa3 => {
                    format!("Set DTR = {} (0x{:02x})", pkt[1], pkt[1])
                }
                0xc3 => {
                    format!("Set DTR1 = {} (0x{:02x})", pkt[1], pkt[1])
                }
                0xc5 => {
                    format!("Set DTR2 = {} (0x{:02x})", pkt[1], pkt[1])
                }
                0xa5 => {
                    format!("Initialise {} (0x{:02x})", pkt[1], pkt[1])
                }
                0xa7 => "Randomise".to_string(),
                0xa9 => "Compare".to_string(),
                0xab => "Withdraw".to_string(),
                0xb1 => format!("Search address high 0x{:02x}", pkt[1]),
                0xb3 => format!("Search address middle 0x{:02x}", pkt[1]),
                0xb5 => format!("Search address low 0x{:02x}", pkt[1]),
                0xb7 => format!("Program short address {}", (pkt[1] >> 1) & 0x3f),
                0xb9 => format!("Verify short address {}", (pkt[1] >> 1) & 0x3f),
                0xbb => "Query short address".to_string(),
                0xbd => "Physical selection".to_string(),
                0xc1 => format!("Enable device type {}", pkt[1]),
                0xc7 => format!("Write memory location: 0x{:02x}", pkt[1]),
                0xc9 => format!("Write memory location (no reply): 0x{:02x}", pkt[1]),
                _ => "Unknown special command".to_string(),
            }
        } else {
            str = decode_addr(pkt[0]) + ": " + CMD_DESCR_16[usize::from(pkt[1])];
        }
    } else {
        str = decode_addr(pkt[0]) + ": " + &format!("Set power = {}", pkt[1]);
    }
    str
}

fn decode_cmd_addr_24bit(addr: u8) -> Option<String> {
    if addr & 0xfe == 0xfe {
        // Broadcast
        Some("Broadcast".to_string())
    } else if addr & 0xfe == 0xfc {
        // Broadcast unaddressed
	Some("Unaddressed".to_string())
    } else if addr & 0xc0 == 0x80 {
        // Group
        Some(format!("Group: {}", (addr >> 1) & 0x0f))
    } else if (addr & 0x80) == 0x00 {
        // Address
        Some(format!("Addr: {}", (addr >> 1) & 0x3f))
    } else {
        None
    }
}

fn decode_device_command(cmd: u8) -> String {
    device_cmd_descr_24(cmd).to_string()
}

fn decode_instance_command(cmd: u8) -> String {
    instance_cmd_descr_24(cmd).to_string()
}

fn decode_instance_type(instance_type: u8) -> &'static str {
    match instance_type {
        1 => "Push button",
        3 => "Occupancy sensor",
        4 => "Light sensor",
        _ => "Unknown",
    }
}

fn source_device_addr(addr: u8) -> String {
    format!("Device addr: {}, ", addr)
}

fn source_device_group(group: u8) -> String {
    format!("Device group: {}, ", group)
}

fn source_instance(instance: u8) -> String {
    format!("Instance: {}, ", instance)
}

fn source_instance_group(group: u8) -> String {
    format!("Instance group: {}, ", group)
}

fn source_instance_type(instance_type: u8) -> String {
    format!("Instance type: {}", decode_instance_type(instance_type))
}

fn decode_event_source(source: &[u8]) -> String {
    assert!(source.len() >= 2);
    if (source[0] & 1) != 0 {
        return "Illegal event".to_string();
    }
    let source1 = (source[0] >> 1) & 0x3f;
    let source2 = (source[1] >> 2) & 0x1f;
    match (source[0] & 0xc1, source[1] & 0x80) {
        (0x00, 0x00) | (0x40, 0x00) => {
            source_device_addr(source1) + ", " + &source_instance_type(source2)
        }
        (0x00, 0x80) | (0x40, 0x80) => {
            source_device_addr(source1) + ", " + &source_instance(source2)
        }
        (0x80, 0x00) => source_device_group(source1) + ", " + &source_instance_type(source2),
        (0x80, 0x80) => source_device_group(source1) + ", " + &source_instance(source2),
        (0xc0, 0x00) => {
            source_instance_group(source1 & 0x1f) + ", " + &source_instance_type(source2)
        }
        _ => "Reserved".to_string(),
    }
}

fn decode_special_command(pkt: &[u8]) -> String {
    assert!(pkt.len() >= 3);
    match pkt[0] {
        0xc1 => match pkt[1] {
            0x00 if pkt[2] == 0 => "Terminate".to_string(),
            0x01 => {
                let device = if pkt[2] == 0x7f {
                    "uninitialized".to_string()
                } else if pkt[2] == 0xff {
                    "all".to_string()
                } else if (pkt[2] & 0xc0) == 0x00 {
                    format!("{} (0x{:02x}", pkt[2], pkt[2])
                } else {
                    "none".to_string()
                };
                "Initialise ".to_string() + &device
            }
            0x02 if pkt[2] == 0x00 => "Randomise".to_string(),
            0x03 if pkt[2] == 0x00 => "Compare".to_string(),
            0x04 if pkt[2] == 0x00 => "Withdraw".to_string(),
            0x05 => format!("Search address high 0x{:02x}", pkt[2]),
            0x06 => format!("Search address middle 0x{:02x}", pkt[2]),
            0x07 => format!("Search address low 0x{:02x}", pkt[2]),
            0x08 => format!("Program short address {}", pkt[2]),
            0x09 => format!("Verify short address {}", pkt[2]),
            0x0a if pkt[2] == 0x00 => "Query short address".to_string(),
            0x20 => format!("Write memory location. Data: {}", pkt[2]),
            0x21 => {
                format!("Write memory location - no reply, data {}", pkt[2])
            }
            0x30 => {
                format!("Set DTR = {} (0x{:02x})", pkt[2], pkt[2])
            }
            0x31 => {
                format!("Set DTR1 = {} (0x{:02x})", pkt[2], pkt[2])
            }
            0x32 => {
                format!("Set DTR2 = {} (0x{:02x})", pkt[2], pkt[2])
            }
            0x33 => {
                format!(
                    "Send testframe. {}, priority {}{}{}",
                    if (pkt[2] & 0x20) == 0 {
                        "24 bits"
                    } else {
                        "16 bits"
                    },
                    pkt[2] & 0x07,
                    if (pkt[2] & 0x40) == 0x40 {
                        " ,transaction"
                    } else {
                        ""
                    },
                    if (pkt[2] & 0x18) > 0 {
                        format!(" ,repeat {} times", (pkt[2] & 0x18) >> 3)
                    } else {
                        "".to_string()
                    }
                )
            }
            _ => "Unknown special command".to_string(),
        },
        0xc5 => format!("Direct write memory, offset {}, data {}", pkt[1], pkt[2]),
        0xc7 => format!(
            "Set DTR1 = {} (0x{:02x}),DTR0 = {} (0x{:02x})",
            pkt[1], pkt[1], pkt[2], pkt[2]
        ),
        0xc9 => format!(
            "Set DTR2 = {} (0x{:02x}),DTR1 = {} (0x{:02x})",
            pkt[1], pkt[1], pkt[2], pkt[2]
        ),

        _ => "Unknown command".to_string(),
    }
}

fn decode_24bit(pkt: &[u8]) -> String {
    let str;
    if (pkt[0] & 1) == 1 {
        // Command frame
        let addr = decode_cmd_addr_24bit(pkt[0]);
        if let Some(addr_str) = addr {
            if pkt[1] == 0xfe {
                // Device command
                str = addr_str + ": " + &decode_device_command(pkt[2]);
            } else {
                // Instance command
                str = addr_str + ": " + &decode_instance_command(pkt[2]);
            }
        } else {
            str = decode_special_command(pkt);
        }
    } else {
        let value = ((u16::from(pkt[1]) & 0x03) << 8) | u16::from(pkt[2]);
        str = "(".to_string()
            + &decode_event_source(&pkt[0..2])
            + "): "
            + &format!("{} (0x{:03x})", value, value);
    }
    str
}

pub fn decode_packet(pkt: &[u8]) -> String {
    let len = pkt.len();
    match len {
        3 => decode_24bit(pkt),
        2 => decode_16bit(pkt),
        _ => "Invalid packet length".to_string(),
    }
}
