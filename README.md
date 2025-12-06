# Sentinel CLI

CLI interactiva en Rust para encender una VPN local y bloquear anuncios sin romper la red. Diseñada con módulos pequeños y mantenibles.

## Características
- Menú interactivo (`sentinel`) sin argumentos.
- VPN local embebida: crea interfaz TUN, aplica NAT con PF y enruta todo el tráfico saliente.
- Bloquea anuncios con DNS local (NXDOMAIN) y segmenta hosts con respaldo automático.
- Blocklist base incluida y soporte para una lista personalizada en `~/.config/sentinel/blocklist.txt`.
- Autoconfiguración en el primer arranque: crea `~/.config/sentinel`, copia la blocklist base y verifica herramientas VPN disponibles.

## Requisitos
- Rust 1.72+ y `cargo`.
- macOS con permisos de administrador (`sudo`) para crear TUN, modificar PF, rutas y DNS.

## Instalación
```bash
cargo install --path .
```
Esto deja disponible el binario `sentinel` en tu `$PATH`.

## Uso
1. Ejecuta `sudo sentinel`.
2. Sigue el menú para:
   - Encender/Apagar la VPN local (TUN + NAT + DNS local).
   - Activar/Desactivar el bloqueo de anuncios.
   - Ver el estado actual.

## Configuración opcional
- **Blocklist personalizada:** crea `~/.config/sentinel/blocklist.txt` con un dominio por línea. Sentinel combina esta lista con la base incluida.
- **Red interna:** por defecto usa 10.99.0.0/24, TUN `utun*` con 10.99.0.1↔10.99.0.2, MTU 1420 y DNS local 127.0.0.1:5353.

## Notas de seguridad
- Sentinel hace copia de `hosts` en `~/.config/sentinel/hosts.bak` antes de añadir entradas. Al desactivar el bloqueador restaura el respaldo cuando existe.
- Requiere `sudo` para levantar la VPN local: crea interfaz TUN, añade rutas, habilita IP forwarding, ajusta DNS y carga reglas PF en un ancla propia (`sentinel`).
- Todas las reglas PF y ajustes de DNS/IP forwarding se revierten al apagar la VPN desde el menú.
