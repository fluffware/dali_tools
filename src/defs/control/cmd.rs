pub struct AddressByte(u8);
pub struct InstanceByte(u8);
pub struct OpcodeByte(u8);

impl From<u8> for AddressByte {
    fn from(b: u8) -> AddressByte {
        AddressByte(b)
    }
}
impl Into<u8> for AddressByte {
    fn into(self) -> u8 {
        self.0
    }
}
impl From<u8> for InstanceByte {
    fn from(b: u8) -> InstanceByte {
        InstanceByte(b)
    }
}

impl Into<u8> for InstanceByte {
    fn into(self) -> u8 {
        self.0
    }
}
impl From<u8> for OpcodeByte {
    fn from(b: u8) -> OpcodeByte {
        OpcodeByte(b)
    }
}

impl Into<u8> for OpcodeByte {
    fn into(self) -> u8 {
        self.0
    }
}

pub struct ControlCommand((AddressByte, InstanceByte, OpcodeByte));

impl ControlCommand {
    const fn new(a: u8, i: u8, o: u8) -> Self {
        ControlCommand((AddressByte(a), InstanceByte(i), OpcodeByte(o)))
    }
}

impl From<(AddressByte, (InstanceByte, OpcodeByte))> for ControlCommand {
    fn from(dev: (AddressByte, (InstanceByte, OpcodeByte))) -> ControlCommand {
        ControlCommand((dev.0, dev.1 .0, dev.1 .1))
    }
}
impl From<(u8, u8, u8)> for ControlCommand {
    fn from(cmd: (u8, u8, u8)) -> ControlCommand {
        ControlCommand((AddressByte(cmd.0), InstanceByte(cmd.1), OpcodeByte(cmd.2)))
    }
}
impl From<(AddressByte, InstanceByte, OpcodeByte)> for ControlCommand {
    fn from(cmd: (AddressByte, InstanceByte, OpcodeByte)) -> ControlCommand {
        ControlCommand(cmd)
    }
}

macro_rules! dev_cmd_def {
    ($sym: ident, $opcode: expr) => {
        pub const $sym: (InstanceByte, OpcodeByte) = (InstanceByte(0xfe), OpcodeByte($opcode));
    };
}

macro_rules! inst_cmd_def {
    ($sym: ident, $opcode: expr) => {
        pub const $sym: OpcodeByte = OpcodeByte($opcode);
    };
}

macro_rules! special_cmd_def {
    ($sym: ident, $opcode: expr) => {
        pub const $sym: (AddressByte, InstanceByte) = (AddressByte(0xc1), InstanceByte($opcode));
    };
}

dev_cmd_def!(IDENTIFY_DEVICE, 0x00);
dev_cmd_def!(RESET_POWER_CYCLE_SEEN, 0x01);
dev_cmd_def!(RESET, 0x10);
dev_cmd_def!(RESET_MEMORY_BANK, 0x11);
dev_cmd_def!(SET_SHORT_ADDRESS, 0x14);
dev_cmd_def!(ENABLE_WRITE_MEMORY, 0x15);
dev_cmd_def!(ENABLE_APPLICATION_CONTROLLER, 0x16);
dev_cmd_def!(DISABLE_APPLICATION_CONTROLLER, 0x17);
dev_cmd_def!(SET_OPERATING_MODE, 0x18);
dev_cmd_def!(ADD_TO_DEVICE_GROUPS_0_15, 0x19);
dev_cmd_def!(ADD_TO_DEVICE_GROUPS_16_31, 0x1a);
dev_cmd_def!(REMOVE_FROM_DEVICE_GROUPS_0_15, 0x1b);
dev_cmd_def!(REMOVE_FROM_DEVICE_GROUPS_16_31, 0x1c);
dev_cmd_def!(START_QUIESCENT_MODE, 0x1d);
dev_cmd_def!(STOP_QUIESCENT_MODE, 0x1e);
dev_cmd_def!(ENABLE_POWER_CYCLE_NOTIFICATION, 0x1f);
dev_cmd_def!(DISABLE_POWER_CYCLE_NOTIFICATION, 0x20);
dev_cmd_def!(SAVE_PERSISTENT_VARIABLES, 0x21);
dev_cmd_def!(QUERY_DEVICE_STATUS, 0x30);
dev_cmd_def!(QUERY_APPLICATION_CONTROLLER_ERROR, 0x31);
dev_cmd_def!(QUERY_INPUT_DEVICE_ERROR, 0x32);
dev_cmd_def!(QUERY_MISSING_SHORT_ADDRESS, 0x33);
dev_cmd_def!(QUERY_VERSION_NUMBER, 0x34);
dev_cmd_def!(QUERY_NUMBER_OF_INSTANCES, 0x35);
dev_cmd_def!(QUERY_CONTENT_DTR0, 0x36);
dev_cmd_def!(QUERY_CONTENT_DTR1, 0x37);
dev_cmd_def!(QUERY_CONTENT_DTR2, 0x38);
dev_cmd_def!(QUERY_RANDOM_ADDRESS_H, 0x39);
dev_cmd_def!(QUERY_RANDOM_ADDRESS_M, 0x3a);
dev_cmd_def!(QUERY_RANDOM_ADDRESS_L, 0x3b);
dev_cmd_def!(READ_MEMORY_LOCATION, 0x3c);
dev_cmd_def!(QUERY_APPLICATION_CONTROL_ENABLED, 0x3d);
dev_cmd_def!(QUERY_OPERATING_MODE, 0x3e);
dev_cmd_def!(QUERY_MANUFACTURER_SPECIFIC_MODE, 0x3f);
dev_cmd_def!(QUERY_QUIESCENT_MODE, 0x40);
dev_cmd_def!(QUERY_DEVICE_GROUPS_0_7, 0x41);
dev_cmd_def!(QUERY_DEVICE_GROUPS_8_15, 0x42);
dev_cmd_def!(QUERY_DEVICE_GROUPS_16_23, 0x43);
dev_cmd_def!(QUERY_DEVICE_GROUPS_24_31, 0x44);
dev_cmd_def!(QUERY_POWER_CYCLE_NOTIFICATION, 0x45);
dev_cmd_def!(QUERY_DEVICE_CAPABILITIES, 0x46);
dev_cmd_def!(QUERY_EXTENDED_VERSION_NUMBER, 0x47);
dev_cmd_def!(QUERY_RESET_STATE, 0x48);

inst_cmd_def!(SET_EVENT_PRIORITY, 0x61);
inst_cmd_def!(ENABLE_INSTANCE, 0x62);
inst_cmd_def!(DISABLE_INSTANCE, 0x63);
inst_cmd_def!(SET_PRIMARY_INSTANCE_GROUP, 0x64);
inst_cmd_def!(SET_INSTANCE_GROUP_1, 0x65);
inst_cmd_def!(SET_INSTANCE_GROUP_2, 0x66);
inst_cmd_def!(SET_EVENT_SCHEME, 0x67);
inst_cmd_def!(SET_EVENT_FILTER, 0x68);

inst_cmd_def!(QUERY_INSTANCE_TYPE, 0x80);
inst_cmd_def!(QUERY_RESOLUTION, 0x81);
inst_cmd_def!(QUERY_INSTANCE_ERROR, 0x82);
inst_cmd_def!(QUERY_INSTANCE_STATUS, 0x83);
inst_cmd_def!(QUERY_EVENT_PRIORITY, 0x84);
inst_cmd_def!(QUERY_INSTANCE_ENABLED, 0x86);
inst_cmd_def!(QUERY_PRIMARY_INSTANCE_GROUP, 0x88);
inst_cmd_def!(QUERY_INSTANCE_GROUP_1, 0x89);
inst_cmd_def!(QUERY_INSTANCE_GROUP_2, 0x8a);
inst_cmd_def!(QUERY_EVENT_SCHEME, 0x8b);
inst_cmd_def!(QUERY_INPUT_VALUE, 0x8c);
inst_cmd_def!(QUERY_INPUT_VALUE_LATCH, 0x8d);
inst_cmd_def!(QUERY_FEATURE_TYPE, 0x8e);
inst_cmd_def!(QUERY_NEXT_FEATURE_TYPE, 0x8f);
inst_cmd_def!(QUERY_EVENT_FILTER_0_7, 0x90);
inst_cmd_def!(QUERY_EVENT_FILTER_8_15, 0x91);
inst_cmd_def!(QUERY_EVENT_FILTER_16_23, 0x92);

pub const TERMINATE: ControlCommand = ControlCommand::new(0xc1, 0x00, 0x00);

special_cmd_def!(INITIALISE, 0x01);
pub const RANDOMISE: ControlCommand = ControlCommand::new(0xc1, 0x02, 0x00);
pub const COMPARE: ControlCommand = ControlCommand::new(0xc1, 0x03, 0x00);
pub const WITHDRAW: ControlCommand = ControlCommand::new(0xc1, 0x04, 0x00);
special_cmd_def!(SEARCHADDRH, 0x05);
special_cmd_def!(SEARCHADDRM, 0x06);
special_cmd_def!(SEARCHADDRL, 0x07);
special_cmd_def!(PROGRAM_SHORT_ADDRESS, 0x08);
special_cmd_def!(VERIFY_SHORT_ADDRESS, 0x09);
pub const QUERY_SHORT_ADDRESS: ControlCommand = ControlCommand::new(0xc1, 0x0a, 0x00);
special_cmd_def!(WRITE_MEMORY_LOCATION, 0x20);
special_cmd_def!(WRITE_MEMORY_LOCATION_NO_REPLY, 0x21);
special_cmd_def!(DTR0, 0x30);
special_cmd_def!(DTR1, 0x31);
special_cmd_def!(DTR2, 0x32);
special_cmd_def!(SEND_TESTFRAME, 0x33);
pub const DIRECT_WRITE_MEMORY: AddressByte = AddressByte(0xc5);
pub const DTR1_DTR0: AddressByte = AddressByte(0xc7);
pub const DTR2_DTR1: AddressByte = AddressByte(0xc9);
