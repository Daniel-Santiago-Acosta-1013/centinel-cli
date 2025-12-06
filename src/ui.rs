use crate::app::SentinelApp;
use anyhow::Result;
use dialoguer::{Select, theme::ColorfulTheme};
use log::{error, info};

pub fn run(app: &mut SentinelApp) -> Result<()> {
    banner();
    loop {
        match menu()? {
            MenuChoice::StartVpn => handle(app, "Activando VPN", |a| a.vpn.start())?,
            MenuChoice::StopVpn => handle(app, "Desactivando VPN", |a| a.vpn.stop())?,
            MenuChoice::StartAdblock => {
                handle(app, "Activando bloqueador", |a| a.adblock.enable())?
            }
            MenuChoice::StopAdblock => {
                handle(app, "Desactivando bloqueador", |a| a.adblock.disable())?
            }
            MenuChoice::Status => show_status(app),
            MenuChoice::Quit => {
                println!("Hasta luego. Mantén la red limpia 👋");
                break;
            }
        }
    }
    Ok(())
}

fn banner() {
    println!(
        "
   ______            __  _      __
  / ____/___  ____  / /_(_)____/ /____  _____
 / /   / __ \\/ __ \\/ __/ / ___/ __/ _ \\/ ___/
/ /___/ /_/ / / / / /_/ / /__/ /_/  __/ /
\\____/\\____/_/ /_/\\__/_/\\___/\\__/\\___/_/   sentinel
"
    );
}

#[derive(Copy, Clone)]
enum MenuChoice {
    StartVpn,
    StopVpn,
    StartAdblock,
    StopAdblock,
    Status,
    Quit,
}

fn menu() -> Result<MenuChoice> {
    let items = vec![
        "Encender VPN",
        "Apagar VPN",
        "Activar bloqueo de anuncios",
        "Desactivar bloqueo de anuncios",
        "Ver estado",
        "Salir",
    ];

    let selection = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("¿Qué deseas hacer?")
        .items(&items)
        .default(0)
        .interact()?;

    let choice = match selection {
        0 => MenuChoice::StartVpn,
        1 => MenuChoice::StopVpn,
        2 => MenuChoice::StartAdblock,
        3 => MenuChoice::StopAdblock,
        4 => MenuChoice::Status,
        _ => MenuChoice::Quit,
    };
    Ok(choice)
}

fn handle<F>(app: &mut SentinelApp, label: &str, action: F) -> Result<()>
where
    F: FnOnce(&mut SentinelApp) -> Result<()>,
{
    println!("{label}...");
    match action(app) {
        Ok(_) => {
            info!("{label} - ok");
            println!("✓ {label}");
        }
        Err(err) => {
            error!("{label} - {err}");
            eprintln!("⚠️  {label} falló: {err}");
        }
    };
    Ok(())
}

fn show_status(app: &SentinelApp) {
    let status = app.status();
    println!();
    println!("Estado actual:");
    println!("• VPN: {}", status.vpn.as_str());
    println!("• Bloqueo de anuncios: {}", status.adblock.as_str());
    println!();
}
