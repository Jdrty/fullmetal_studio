//! open a USB UART at 8N1 for talking to a programmed AVR

use serialport::{DataBits, FlowControl, Parity, StopBits};
use std::time::Duration;

pub fn open(path: &str, baud: u32) -> Result<Box<dyn serialport::SerialPort>, String> {
    serialport::new(path, baud)
        .data_bits(DataBits::Eight)
        .parity(Parity::None)
        .stop_bits(StopBits::One)
        .flow_control(FlowControl::None)
        .timeout(Duration::from_millis(2))
        .open()
        .map_err(|e| e.to_string())
}
