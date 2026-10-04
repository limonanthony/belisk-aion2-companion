// The shipped exe opens no console window. Probes builds keep theirs, because
// their results are printed.
#![cfg_attr(not(feature = "probes"), windows_subsystem = "windows")]

fn main() -> std::process::ExitCode {
    aion_2_dps_meter::run()
}
