#![no_std]

#[derive(Debug, PartialEq, Eq)]
pub enum Command {
    Up,
    Down,
    Get,
}

/// EOUTLET/1 <device-id> <UP|DOWN|GET>; reject all extra fields.
pub fn parse_command(packet: &[u8], device_id: &str) -> Option<Command> {
    let text = core::str::from_utf8(packet).ok()?;
    let mut fields = text.split_ascii_whitespace();
    if fields.next()? != "EOUTLET/1" || fields.next()? != device_id {
        return None;
    }
    let command = match fields.next()? {
        "UP" => Command::Up,
        "DOWN" => Command::Down,
        "GET" => Command::Get,
        _ => return None,
    };
    if fields.next().is_some() {
        return None;
    }
    Some(command)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_addressed_commands_and_line_endings() {
        assert_eq!(
            parse_command(b"EOUTLET/1 outlet-123 UP\r\n", "outlet-123"),
            Some(Command::Up)
        );
        assert_eq!(
            parse_command(b"EOUTLET/1 outlet-123 DOWN", "outlet-123"),
            Some(Command::Down)
        );
        assert_eq!(
            parse_command(b"EOUTLET/1 outlet-123 GET", "outlet-123"),
            Some(Command::Get)
        );
    }

    #[test]
    fn rejects_other_devices_malformed_and_unknown_packets() {
        for packet in [
            &b"EOUTLET/1 other UP"[..],
            &b"EOUTLET/2 outlet-123 UP"[..],
            &b"EOUTLET/1 outlet-123 TOGGLE"[..],
            &b"EOUTLET/1 outlet-123 UP extra"[..],
            &b"EOUTLET/1 outlet-123"[..],
            &b""[..],
            &[255][..],
        ] {
            assert_eq!(parse_command(packet, "outlet-123"), None);
        }
    }
}
