# Sentinel CLI

CLI interactiva en Rust para encender una VPN local y bloquear anuncios sin romper la red. Diseñada con módulos pequeños y mantenibles.

## Características
- Menú interactivo (`sentinel`) sin argumentos.
- Enciende/apaga VPN usando las herramientas disponibles (`wg-quick`, `scutil/nmcli/rasdial`, `openvpn`).
- Bloquea anuncios añadiendo un segmento aislado en el archivo `hosts` con respaldo automático.
- Blocklist base incluida y soporte para una lista personalizada en `~/.config/sentinel/blocklist.txt`.

## Requisitos
- Rust 1.72+ y `cargo`.
- Permisos de administrador para modificar `hosts` o levantar interfaces VPN.
- Herramienta de VPN instalada en el sistema (WireGuard, NetworkManager, OpenVPN o equivalente).

## Instalación
```bash
cargo install --path .
```
Esto deja disponible el binario `sentinel` en tu `$PATH`.

## Uso
1. Ejecuta `sentinel`.
2. Sigue el menú para:
   - Encender/Apagar la VPN.
   - Activar/Desactivar el bloqueo de anuncios.
   - Ver el estado actual.

## Configuración opcional
- **Blocklist personalizada:** crea `~/.config/sentinel/blocklist.txt` con un dominio por línea. Sentinel combina esta lista con la base incluida.
- **VPN:** coloca tu perfil WireGuard llamado `sentinel` (`wg-quick up sentinel`) o un `sentinel.ovpn` en `~/.config/sentinel/vpn/`. En macOS se intentará `scutil --nc start SentinelVPN`; en Linux `nmcli connection up SentinelVPN`.

## Notas de seguridad
- Sentinel hace copia de `hosts` en `~/.config/sentinel/hosts.bak` antes de añadir entradas. Al desactivar el bloqueador restaura el respaldo cuando existe.
- Si recibes errores de permisos, vuelve a ejecutar con `sudo sentinel`.
