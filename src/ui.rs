use crate::app::SentinelApp;
use crate::setup::SetupReport;
use anyhow::Result;
use crossterm::{
    cursor::MoveTo,
    execute,
    terminal::{Clear, ClearType},
};
use dialoguer::{Select, theme::ColorfulTheme};
use log::{error, info};
use std::io::{Write, stdin, stdout};

pub fn run(app: &mut SentinelApp, setup: &SetupReport) -> Result<()> {
    let mut first_screen = true;
    loop {
        clear_screen()?;
        banner();
        if first_screen {
            print_setup(setup);
            first_screen = false;
        }
        show_brief(app);
        match menu()? {
            MenuChoice::StartVpn => {
                handle(app, "Activando VPN local", |a| a.vpn.start())?;
                wait_for_enter();
            }
            MenuChoice::StopVpn => {
                handle(app, "Desactivando VPN", |a| a.vpn.stop())?;
                wait_for_enter();
            }
            MenuChoice::StartAdblock => {
                handle(app, "Activando bloqueo de anuncios", |a| a.adblock.enable())?;
                wait_for_enter();
            }
            MenuChoice::StopAdblock => {
                handle(app, "Desactivando bloqueo de anuncios", |a| {
                    a.adblock.disable()
                })?;
                wait_for_enter();
            }
            MenuChoice::Status => {
                show_status(app);
                wait_for_enter();
            }
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

fn clear_screen() -> Result<()> {
    execute!(stdout(), Clear(ClearType::All), MoveTo(0, 0))?;
    Ok(())
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

fn print_setup(setup: &SetupReport) {
    println!("Config dir: {}", setup.config_dir.display());
    if let Some(path) = &setup.blocklist_path {
        if setup.created_blocklist {
            println!("Blocklist inicial creada en {}", path.display());
        } else {
            println!("Blocklist personalizada encontrada en {}", path.display());
        }
    }
    let vpn_dir = setup.config_dir.join("vpn");
    if setup.created_vpn_dir {
        println!(
            "Se creó la carpeta VPN local en {} (archivo sentinel.conf generado automáticamente).",
            vpn_dir.display()
        );
    } else {
        println!(
            "Carpeta VPN local: {} (usa sentinel.conf para ajustes avanzados).",
            vpn_dir.display()
        );
    }
    println!(
        "Ruta de configuración de la VPN embebida: {}",
        setup.vpn_config_path.display()
    );
    println!("Herramientas VPN detectadas: {}", setup.tools.summary());
    if !setup.tools.any() {
        println!("→ No se detecta soporte embebido (debería ser 'embebida').");
    }
    println!();
}

fn show_brief(app: &SentinelApp) {
    let status = app.status();
    println!(
        "VPN: {} | Anuncios: {} | Herramientas: {}",
        status.vpn.as_str(),
        status.adblock.as_str(),
        app.tools.summary()
    );
    println!();
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

fn wait_for_enter() {
    print!("Presiona Enter para continuar...");
    let _ = stdout().flush();
    let mut buffer = String::new();
    let _ = stdin().read_line(&mut buffer);
}
