use std::process::Command;

pub fn execute_action(action: &str) -> Result<(), String> {
    match action {
        "shutdown" => {
            Command::new("shutdown")
                .args(["-s", "-t", "0"])
                .spawn()
                .map_err(|e| e.to_string())?;
        }
        "restart" => {
            Command::new("shutdown")
                .args(["-r", "-t", "0"])
                .spawn()
                .map_err(|e| e.to_string())?;
        }
        "sleep" => {
            Command::new("rundll32.exe")
                .args(["powrprof.dll,SetSuspendState", "0", "1", "0"])
                .spawn()
                .map_err(|e| e.to_string())?;
        }
        _ => {
            return Err("Invalid action".to_string());
        }
    }
    Ok(())
}
