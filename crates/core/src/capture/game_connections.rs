use std::fmt;
use std::net::Ipv4Addr;

const GAME_IMAGE_NAME: &str = "aion2.exe";

const TCP_STATE_LISTEN: u32 = 2;
const TCP_STATE_DELETE_TCB: u32 = 12;

const HTTPS_PORT: u16 = 443;
const HTTP_PORT: u16 = 80;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct GameConnection {
    pub local_addr: Ipv4Addr,
    pub local_port: u16,
    pub remote_addr: Ipv4Addr,
    pub remote_port: u16,
}

impl GameConnection {
    pub fn goes_through_loopback(&self) -> bool {
        self.remote_addr == Ipv4Addr::LOCALHOST
    }

    fn is_peer_of(&self, other: &GameConnection) -> bool {
        self.remote_addr == other.local_addr && self.remote_port == other.local_port
    }
}

impl fmt::Display for GameConnection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}:{} -> {}:{}",
            self.local_addr, self.local_port, self.remote_addr, self.remote_port
        )
    }
}

/// One row of the system TCP table with the fields exactly as Windows reports
/// them: addresses and ports in network byte order.
#[derive(Debug, Clone, Copy)]
struct TcpRow {
    state: u32,
    local_addr: u32,
    local_port: u32,
    remote_addr: u32,
    remote_port: u32,
    owning_pid: u32,
}

fn is_game_image_name(image_name: &str) -> bool {
    image_name.eq_ignore_ascii_case(GAME_IMAGE_NAME)
}

fn ipv4_from_network_order(value: u32) -> Ipv4Addr {
    Ipv4Addr::from(value.to_ne_bytes())
}

fn port_from_network_order(value: u32) -> u16 {
    u16::from_be(value as u16)
}

fn is_live_state(state: u32) -> bool {
    state != TCP_STATE_LISTEN && state != TCP_STATE_DELETE_TCB
}

fn is_web_port(port: u16) -> bool {
    port == HTTPS_PORT || port == HTTP_PORT
}

fn connections_of_processes(rows: &[TcpRow], process_ids: &[u32]) -> Vec<GameConnection> {
    let game_connections: Vec<GameConnection> = rows
        .iter()
        .filter(|row| process_ids.contains(&row.owning_pid) && is_live_state(row.state))
        .map(|row| GameConnection {
            local_addr: ipv4_from_network_order(row.local_addr),
            local_port: port_from_network_order(row.local_port),
            remote_addr: ipv4_from_network_order(row.remote_addr),
            remote_port: port_from_network_order(row.remote_port),
        })
        .collect();
    let mut connections: Vec<GameConnection> = game_connections
        .iter()
        .filter(|connection| !is_web_port(connection.remote_port))
        .filter(|connection| {
            !(connection.goes_through_loopback()
                && game_connections.iter().any(|peer| connection.is_peer_of(peer)))
        })
        .copied()
        .collect();
    connections.sort();
    connections.dedup();
    connections
}

/// The game's live IPv4 TCP connections, sorted.
///
/// Reads the process list by image name (like Task Manager) and the system TCP
/// table (like `netstat -ano`). It opens no handle on any process.
pub fn query_game_connections() -> Result<Vec<GameConnection>, String> {
    let process_ids = os::game_process_ids()?;
    if process_ids.is_empty() {
        return Ok(Vec::new());
    }
    let rows = os::tcp_rows()?;
    Ok(connections_of_processes(&rows, &process_ids))
}

mod os {
    use std::ffi::c_void;
    use std::mem::size_of;

    use windows::Win32::Foundation::{CloseHandle, ERROR_INSUFFICIENT_BUFFER, NO_ERROR};
    use windows::Win32::NetworkManagement::IpHelper::{
        GetExtendedTcpTable, MIB_TCPROW_OWNER_PID, TCP_TABLE_OWNER_PID_ALL,
    };
    use windows::Win32::Networking::WinSock::AF_INET;
    use windows::Win32::System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W,
        TH32CS_SNAPPROCESS,
    };

    use super::{is_game_image_name, TcpRow};

    const TABLE_FILL_ATTEMPTS: usize = 4;

    pub fn game_process_ids() -> Result<Vec<u32>, String> {
        let snapshot = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) }
            .map_err(|error| format!("CreateToolhelp32Snapshot failed: {error}"))?;

        let mut process_ids = Vec::new();
        let mut entry = PROCESSENTRY32W {
            dwSize: size_of::<PROCESSENTRY32W>() as u32,
            ..Default::default()
        };
        let mut has_entry = unsafe { Process32FirstW(snapshot, &mut entry) }.is_ok();
        while has_entry {
            let name_len = entry
                .szExeFile
                .iter()
                .position(|&unit| unit == 0)
                .unwrap_or(entry.szExeFile.len());
            if is_game_image_name(&String::from_utf16_lossy(&entry.szExeFile[..name_len])) {
                process_ids.push(entry.th32ProcessID);
            }
            has_entry = unsafe { Process32NextW(snapshot, &mut entry) }.is_ok();
        }

        let _ = unsafe { CloseHandle(snapshot) };
        Ok(process_ids)
    }

    pub fn tcp_rows() -> Result<Vec<TcpRow>, String> {
        let mut size = 0u32;
        let mut buffer: Vec<u32> = Vec::new();

        for _ in 0..TABLE_FILL_ATTEMPTS {
            let buffer_ptr = if buffer.is_empty() {
                None
            } else {
                Some(buffer.as_mut_ptr() as *mut c_void)
            };
            let status = unsafe {
                GetExtendedTcpTable(
                    buffer_ptr,
                    &mut size,
                    false,
                    AF_INET.0 as u32,
                    TCP_TABLE_OWNER_PID_ALL,
                    0,
                )
            };

            if status == NO_ERROR.0 {
                return Ok(read_rows(&buffer));
            }
            if status != ERROR_INSUFFICIENT_BUFFER.0 {
                return Err(format!("GetExtendedTcpTable failed with error {status}"));
            }
            buffer = vec![0u32; (size as usize).div_ceil(size_of::<u32>())];
        }

        Err("GetExtendedTcpTable kept growing between the size probe and the read".to_string())
    }

    fn read_rows(buffer: &[u32]) -> Vec<TcpRow> {
        let Some((&entry_count, row_words)) = buffer.split_first() else {
            return Vec::new();
        };
        let row_words_len = size_of::<MIB_TCPROW_OWNER_PID>() / size_of::<u32>();
        row_words
            .chunks_exact(row_words_len)
            .take(entry_count as usize)
            .map(|row| TcpRow {
                state: row[0],
                local_addr: row[1],
                local_port: row[2],
                remote_addr: row[3],
                remote_port: row[4],
                owning_pid: row[5],
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(state: u32, local: [u8; 4], local_port: u16, remote: [u8; 4], remote_port: u16, pid: u32) -> TcpRow {
        TcpRow {
            state,
            local_addr: u32::from_ne_bytes(local),
            local_port: u32::from(local_port.to_be()),
            remote_addr: u32::from_ne_bytes(remote),
            remote_port: u32::from(remote_port.to_be()),
            owning_pid: pid,
        }
    }

    const ESTABLISHED: u32 = 5;

    #[test]
    fn keeps_only_rows_of_the_game_processes() {
        let rows = [
            row(ESTABLISHED, [192, 168, 1, 20], 50_000, [203, 0, 113, 7], 13_328, 400),
            row(ESTABLISHED, [192, 168, 1, 20], 50_001, [198, 51, 100, 2], 443, 999),
            row(ESTABLISHED, [192, 168, 1, 20], 50_002, [203, 0, 113, 8], 13_328, 401),
        ];

        let connections = connections_of_processes(&rows, &[400, 401]);

        assert_eq!(
            connections.iter().map(|c| c.local_port).collect::<Vec<_>>(),
            vec![50_000, 50_002]
        );
    }

    #[test]
    fn skips_listening_and_deleted_rows_but_keeps_other_live_states() {
        let rows = [
            row(TCP_STATE_LISTEN, [0, 0, 0, 0], 7000, [0, 0, 0, 0], 0, 400),
            row(TCP_STATE_DELETE_TCB, [10, 0, 0, 2], 7001, [10, 0, 0, 9], 13_328, 400),
            row(3, [10, 0, 0, 2], 7002, [10, 0, 0, 9], 13_328, 400),
            row(ESTABLISHED, [10, 0, 0, 2], 7003, [10, 0, 0, 9], 13_328, 400),
            row(8, [10, 0, 0, 2], 7004, [10, 0, 0, 9], 13_328, 400),
        ];

        let connections = connections_of_processes(&rows, &[400]);

        assert_eq!(
            connections.iter().map(|c| c.local_port).collect::<Vec<_>>(),
            vec![7002, 7003, 7004]
        );
    }

    #[test]
    fn converts_addresses_and_ports_from_network_byte_order() {
        let rows = [row(ESTABLISHED, [192, 168, 1, 20], 51_234, [127, 0, 0, 1], 13_328, 400)];

        let connections = connections_of_processes(&rows, &[400]);

        assert_eq!(
            connections,
            vec![GameConnection {
                local_addr: Ipv4Addr::new(192, 168, 1, 20),
                local_port: 51_234,
                remote_addr: Ipv4Addr::LOCALHOST,
                remote_port: 13_328,
            }]
        );
    }

    #[test]
    fn ports_are_read_from_the_low_16_bits_of_the_dword() {
        let mut game_row = row(ESTABLISHED, [192, 168, 1, 20], 51_234, [10, 0, 0, 9], 13_328, 400);
        game_row.local_port |= 0xABCD_0000;
        game_row.remote_port |= 0x1234_0000;

        let connections = connections_of_processes(&[game_row], &[400]);

        assert_eq!(connections[0].local_port, 51_234);
        assert_eq!(connections[0].remote_port, 13_328);
    }

    #[test]
    fn drops_the_games_own_loopback_socket_pair() {
        let rows = [
            row(ESTABLISHED, [192, 168, 0, 139], 51_150, [193, 202, 112, 95], 13_328, 400),
            row(ESTABLISHED, [127, 0, 0, 1], 51_134, [127, 0, 0, 1], 51_135, 400),
            row(ESTABLISHED, [127, 0, 0, 1], 51_135, [127, 0, 0, 1], 51_134, 400),
        ];

        let connections = connections_of_processes(&rows, &[400]);

        assert_eq!(
            connections.iter().map(|c| c.local_port).collect::<Vec<_>>(),
            vec![51_150]
        );
        assert!(connections.iter().all(|c| !c.goes_through_loopback()));
    }

    #[test]
    fn keeps_a_loopback_connection_whose_peer_is_not_owned_by_the_game() {
        let rows = [
            row(ESTABLISHED, [127, 0, 0, 1], 51_140, [127, 0, 0, 1], 9_000, 400),
            row(ESTABLISHED, [127, 0, 0, 1], 9_000, [127, 0, 0, 1], 51_140, 777),
        ];

        let connections = connections_of_processes(&rows, &[400]);

        assert_eq!(connections.len(), 1);
        assert_eq!(connections[0].local_port, 51_140);
        assert!(connections[0].goes_through_loopback());
    }

    #[test]
    fn drops_web_traffic_and_keeps_the_game_port() {
        let rows = [
            row(ESTABLISHED, [192, 168, 0, 139], 51_150, [193, 202, 112, 95], 13_328, 400),
            row(ESTABLISHED, [192, 168, 0, 139], 51_151, [198, 51, 100, 2], 443, 400),
            row(ESTABLISHED, [192, 168, 0, 139], 51_152, [198, 51, 100, 3], 80, 400),
        ];

        let connections = connections_of_processes(&rows, &[400]);

        assert_eq!(
            connections.iter().map(|c| c.remote_port).collect::<Vec<_>>(),
            vec![13_328]
        );
    }

    #[test]
    fn loopback_remote_marks_an_accelerator_connection() {
        let direct = GameConnection {
            local_addr: Ipv4Addr::new(192, 168, 1, 20),
            local_port: 50_000,
            remote_addr: Ipv4Addr::new(203, 0, 113, 7),
            remote_port: 13_328,
        };
        let proxied = GameConnection {
            remote_addr: Ipv4Addr::LOCALHOST,
            ..direct
        };

        assert!(!direct.goes_through_loopback());
        assert!(proxied.goes_through_loopback());
    }

    #[test]
    fn image_name_matches_exactly_and_ignores_case() {
        assert!(is_game_image_name("aion2.exe"));
        assert!(is_game_image_name("AION2.exe"));
        assert!(!is_game_image_name("aion2launcher.exe"));
        assert!(!is_game_image_name("myaion2.exe"));
        assert!(!is_game_image_name("aion2.exe.bak"));
        assert!(!is_game_image_name("aion2"));
    }
}
