# Fix `/dev/nvidia1` EIO (Host Runbook)

Symptom we observed:

- `cross-chain-gpu-validator/scripts/gpu_diagnostics.py` reports `/dev/nvidia1` open fails with `Errno 5` (Input/output error).
- CUDA init fails (`cuInit` / `cudaGetDeviceCount`), so GPU lanes stay `degraded` and `tests/inferstructor/infrastructure_benchmark.py` aborts.

This is a host/driver issue, not a repo code issue. You will need `sudo` on the machine.

## 0) Snapshot Evidence (Before)

Run:

```bash
cd /home/lojak/Desktop/x3-chain-master
cross-chain-gpu-validator/scripts/run_proof_suite.sh
```

Confirm in the newest `cross-chain-gpu-validator/artifacts/proof_*/gpu_diagnostics.json`:

- `/dev/nvidia1` => `open_rw_ok: false` with `open_rw_errno: 5`

## 1) Check GPU1 Is Not In Use

```bash
nvidia-smi
```

If GPU 1 has any running processes, stop them first (Docker containers, miners, Xorg/Wayland, etc.). A reset typically fails while in use.

## 1.1) Check For a Physically Present But Unpowered GPU

If the kernel log shows:

- `GPU does not have the necessary power cables connected`

then this is not a software problem: the GPU is physically present on PCIe but not powered.

Confirm what PCIe GPUs exist:

```bash
lspci -nn | rg -i 'nvidia|vga|3d|display'
```

If you see a GPU in `lspci` that does not appear in `nvidia-smi -L`, that GPU is very likely the culprit.

Fix options:

- Connect the required PCIe power cables to that GPU.
- Remove the GPU from the machine.
- Disable that PCIe slot/device in BIOS/UEFI.

After fixing power/hardware, reboot and re-run:

```bash
nvidia-smi -L
python3 /home/lojak/Desktop/x3-chain-master/cross-chain-gpu-validator/scripts/gpu_diagnostics.py
```

## 2) Attempt a Targeted GPU Reset (Least Disruptive)

This often works if the GPU is idle:

```bash
sudo nvidia-smi -i 1 --gpu-reset
```

If `nvidia-smi` shows `Disp.A` = `On` for GPU 1 or your desktop is using it (Xorg/Wayland), the reset will fail. Use the TTY method below.

### TTY Method (When Desktop Uses GPU 1)

1. Save work.
2. Switch to a text console: `Ctrl` + `Alt` + `F3` (or `F4`).
3. Log in.
4. Stop the display manager (pick the one that exists on your system):

```bash
sudo systemctl stop gdm || true
sudo systemctl stop sddm || true
sudo systemctl stop lightdm || true
```

5. Confirm GPU 1 has no processes:

```bash
nvidia-smi
```

6. Reset GPU 1:

```bash
sudo nvidia-smi -i 1 --gpu-reset
```

7. Reboot (recommended) or restart the display manager:

```bash
sudo reboot
```

If you see “Reset is not supported”, skip to the module reload step or reboot.

## 3) If Reset Fails: Rebind or Reload Driver Modules

Warning: this can disrupt graphics / CUDA on the host.

```bash
sudo systemctl stop docker || true
sudo systemctl stop nvidia-persistenced || true

# If a display manager is running, you may need to stop it (choose one that exists):
sudo systemctl stop gdm || true
sudo systemctl stop sddm || true
sudo systemctl stop lightdm || true

# Reload modules
sudo modprobe -r nvidia_uvm nvidia_drm nvidia_modeset nvidia || true
sudo modprobe nvidia
sudo modprobe nvidia_modeset
sudo modprobe nvidia_drm
sudo modprobe nvidia_uvm
```

If module unload fails because something is using the driver, you either need to stop that service/process or reboot.

## 4) Last Resort: Reboot

```bash
sudo reboot
```

## 5) Verify the Fix

After reset/reload/reboot:

```bash
nvidia-smi -L
python3 /home/lojak/Desktop/x3-chain-master/cross-chain-gpu-validator/scripts/gpu_diagnostics.py | rg -n \"nvidia1|cuda_probe|cuinit|open_rw\" -S
```

Success criteria:

- `/dev/nvidia1` opens read/write (no `Errno 5`)
- `cuda_probe.cuinit_result` is `0`
- `cuda_probe.cudart_get_device_count_err` is `0`
- `cuda_probe.cudart_get_device_count` is `>= 1`

Then re-run:

```bash
cd /home/lojak/Desktop/x3-chain-master
cross-chain-gpu-validator/scripts/run_proof_suite.sh
```

and confirm `health_primary.json` / `health_shadow.json` / `health_tertiary.json` show `status=healthy` and `gpu.available=true`.
