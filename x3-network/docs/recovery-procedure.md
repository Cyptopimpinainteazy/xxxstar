# Console and recovery preparation

Observed USB identity: Silicon Labs CP2104 USB to UART, VID:PID `10c4:ea60`,
adapter serial `011C3902`, Linux driver `cp210x`, device `/dev/ttyUSB0`.
Stable path:
`/dev/serial/by-id/usb-Silicon_Labs_CP2104_USB_to_UART_Bridge_Controller_011C3902-if00-port0`.
This identifies a USB UART, not the attached switch's chassis serial number.

ALE's indexed OS6860 hardware guide and datasheet describe a micro-USB console,
separate USB host and RS232 ports. Use a data-capable host USB to micro-USB cable
for the USB console; do not use USB-A to USB-A or plug the computer into the host
port. Verify the physical label and connector before connection. An external
CP2104 adapter can also enumerate, so enumeration alone does not prove cable wiring.
The E-series guide gives 9600 baud, 8 data bits, no parity, one stop bit.
Use no software or hardware flow control; verify actual installed console settings.
The N-series 115200 default must not be transferred to this E-series unit.

Initial blocker was errno 13 on root:dialout mode 0660. Operator has since granted
a named lojak rw ACL and installed picocom 3.1. The next observed blocker was EIO on opening
the device: kernel cp210x UART-enable request fails with USB -71 (EPROTO).
That failure occurred before baud-rate configuration or switch login. Exact physical cause
is unproven. Reseat the USB console data cable, then test a known-good data cable
and direct USB port if necessary. Do not power-cycle the switch or disrupt Ethernet.
Re-enumeration may remove the temporary ACL. Noninteractive sudo still needs a password.
No attempt was made to bypass permissions or obtain privilege through Docker.

Operator host preparation, in a local terminal (password stays there):

```bash
sudo apt update
sudo apt install -y picocom minicom screen usbutils iproute2 ethtool lldpd tcpdump curl jq git
sudo usermod -aG dialout lojak
```

Start a fresh login/session after group membership changes. Verify `id` and
read/write access, then rerun discovery. Do not restart host networking. Package
installation is NOT RUN here and may activate lldpd; review daemon policy locally.
Do not remove lojak's existing groups (`-aG` is intentional).

After permission is effective and physical cable/model verified:

```bash
picocom --baud 9600 --databits 8 --parity n --stopbits 1 --flow n \
  /dev/serial/by-id/usb-Silicon_Labs_CP2104_USB_to_UART_Bridge_Controller_011C3902-if00-port0
```

Check whether another console process owns the port before opening it. Avoid BREAK,
boot interruption and power cycles. Authenticate interactively without logging
passwords. Use help to identify read-only identity and health commands. Retain raw
configuration in the private state directory, never in this checkout or chat.

If credentials are unknown, stop at the login prompt, identify the installed AOS
release from available boot evidence without rebooting, and obtain the matching ALE
password-recovery procedure. At that initial checkpoint no bootloader or password-recovery command was verified.
The subsequently verified recovery for U3581156 is recorded below. Do not erase firmware, licenses, recovery partitions or credentials.
Recovery must be tested during an approved maintenance window before migration.

Sources and availability limitations are recorded in [reset procedure](reset-procedure.md).

Latest observation (October 21, 2026): the operator restored the named ACL, and
the next open succeeded at 9600/8N1 with no flow control. A carriage return received
`OS6860 login:`. The earlier USB fault was not observed in that attempt. Current
blocker is authentication, not permissions or UART initialization. Use known
credentials interactively in picocom; never place them in a report or chat.
Exit picocom with Ctrl-A, Ctrl-X after successful login without logging out of the
switch to permit subsequent read-only inspection. No default password was tried.

## Observed bootloader recovery interface — October 21, 2026

On the approved isolated spare, interrupting the displayed autoboot countdown
reached `u-boot>`. Device help reports:

```text
setAdminPasswordDefault - Reset AOS admin user password to default
```

The command's own help lists no arguments. It was invoked once and returned to
the prompt without an error. This is not proof of successful password recovery
until an AOS login succeeds. `printenv bootcmd` reported `run ext4boot`; executing
`run bootcmd` follows that observed normal boot path. Boot completion and login
verification are pending. No saveenv, rescue, root-access command, file deletion,
format or image replacement was issued.

The bootloader explicitly warns `run rescue` destroys all switch data and requires
external recovery images. That command is not the selected path. First recover
normal admin access, then inspect the installed release and exact supported
configuration-reset mechanism.

Model-specific corroboration (community guidance, not an installed-release manual):
[Password Recovery 6860E](https://community.spacewalkers.com/t/password-recovery-6860e/2998).
The device's own command help is the verified command reference for this unit.

## Verified recovery result — serial U3581156

Vendor bootloader password recovery succeeded. The approved startup reset was
completed, old custom accounts removed, and a generated administrator password
applied through the concealed interactive password dialogue. The password and
account cleanup were verified by a fresh login and again after a software reboot.
The generated secret is in the 0600 credential file referenced by switches.yaml,
outside Git; do not publish it. Built-in default is a non-login template. Remote
AAA is denied, so administration currently requires the working local console.

Wait for Master/Running before changing accounts: this release initially exposes
a login prompt while still initializing and rejects changes as not ready. On this
unit initialization settled after approximately four minutes. Session exit asks
for confirmation when running config is unsaved; do not mistake that prompt for
a login failure. Named dialout membership now permits new sg dialout processes
to retain serial permissions across cable re-enumeration.

Rollback material remains on-switch in /flash/x3-pre-reset-U3581156-20261021 and
in private terminal exports on x3star1. Restoring old configuration would reintroduce
the prior network setup; it is not an approved deployment action. The other two
units require their own identity, backup, release checks and reset approval.
