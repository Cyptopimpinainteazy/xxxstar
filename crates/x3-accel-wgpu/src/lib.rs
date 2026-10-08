//! wgpu support crate for X3 accelerator backends.
//!
//! This crate owns device/adapter discovery for non-CUDA acceleration. Kernel
//! implementations are deliberately explicit: algorithms without a WGSL kernel
//! return an error instead of silently falling back inside the backend.

use std::num::NonZeroU64;
use std::sync::{mpsc, Mutex, OnceLock};

mod secp256k1;
pub use secp256k1::Secp256k1Prepared;

const WORKGROUP_SIZE: u32 = 64;
/// Both digests are 32 bytes = 8 u32 words.
const DIGEST_WORDS: usize = 8;
/// Two header words (message count, reserved) precede the per-message
/// `(word_offset, byte_len)` pairs in the meta buffer.
const META_HEADER_WORDS: usize = 2;
/// Upper bound on message bytes uploaded per dispatch. Larger batches are split;
/// this keeps per-call staging memory bounded regardless of device limits.
const MAX_DISPATCH_DATA_BYTES: u64 = 64 << 20;
/// A message must leave room for its padding without the kernel's u32 byte
/// arithmetic wrapping.
const MAX_MESSAGE_BYTES: usize = (u32::MAX - 256) as usize;

/// Error returned by the wgpu support layer.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum WgpuAccelError {
    #[error("no wgpu adapter is available")]
    AdapterUnavailable,
    #[error("wgpu device request failed: {0}")]
    DeviceRequestFailed(String),
    #[error("wgpu kernel is not implemented for {0}")]
    KernelUnavailable(&'static str),
    #[error("invalid batch input: {0}")]
    InvalidInput(&'static str),
    #[error("wgpu buffer map failed: {0}")]
    BufferMapFailed(String),
}

/// wgpu backend handle with cached SHA-256 and Keccak-256 compute resources.
pub struct WgpuBackend {
    adapter_info: wgpu::AdapterInfo,
    device: wgpu::Device,
    queue: wgpu::Queue,
    sha256: ComputeKernel,
    keccak: ComputeKernel,
    // Built on first use: the shader is large and hash-only callers never need it.
    secp256k1_kernels: OnceLock<(ComputeKernel, ComputeKernel)>,
}

/// One hash kernel and its reusable buffers.
///
/// Input layout shared by both kernels: `data` holds every message's raw
/// bytes, each starting on a 4-byte boundary and zero-filled to it; `meta`
/// holds `[count, 0, (word_offset, byte_len) * count]`. Padding is computed by
/// the kernel, so the host does one `memcpy` per message straight into staging
/// memory instead of building padded blocks.
pub(crate) struct ComputeKernel {
    label: &'static str,
    /// u32 words of output per item.
    output_words: usize,
    layout: wgpu::BindGroupLayout,
    pipeline: wgpu::ComputePipeline,
    buffers: Mutex<Option<HashBuffers>>,
}

struct HashBuffers {
    data_word_capacity: usize,
    message_capacity: usize,
    data_buffer: wgpu::Buffer,
    meta_buffer: wgpu::Buffer,
    output_buffer: wgpu::Buffer,
    readback_buffer: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
}

impl WgpuBackend {
    /// Try to initialize the default high-performance wgpu adapter.
    pub fn initialize() -> Result<Self, WgpuAccelError> {
        pollster::block_on(Self::initialize_async())
    }

    async fn initialize_async() -> Result<Self, WgpuAccelError> {
        let adapter = instance()
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: None,
                force_fallback_adapter: false,
            })
            .await
            .ok_or(WgpuAccelError::AdapterUnavailable)?;
        // A software rasterizer (llvmpipe/lavapipe) is a CPU wearing a GPU
        // label: slower than the native CPU path and misleading in metrics.
        if adapter.get_info().device_type == wgpu::DeviceType::Cpu {
            return Err(WgpuAccelError::AdapterUnavailable);
        }
        Self::from_adapter(adapter).await
    }

    /// Hardware GPU adapters, in a stable order (vendor, device id, name).
    ///
    /// Software rasterizers (llvmpipe) are excluded: a "GPU" result computed
    /// on the CPU would make a parity or throughput run meaningless. The
    /// index into this list is what [`WgpuBackend::initialize_adapter`] takes.
    pub fn hardware_adapters() -> Vec<wgpu::AdapterInfo> {
        hardware_adapters_raw()
            .into_iter()
            .map(|adapter| adapter.get_info())
            .collect()
    }

    /// Initialize on one specific hardware adapter from [`Self::hardware_adapters`].
    pub fn initialize_adapter(index: usize) -> Result<Self, WgpuAccelError> {
        let adapter = hardware_adapters_raw()
            .into_iter()
            .nth(index)
            .ok_or(WgpuAccelError::AdapterUnavailable)?;
        pollster::block_on(Self::from_adapter(adapter))
    }

    async fn from_adapter(adapter: wgpu::Adapter) -> Result<Self, WgpuAccelError> {
        let adapter_info = adapter.get_info();
        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("x3-accel-wgpu"),
                    required_features: wgpu::Features::empty(),
                    // The adapter's own limits: downlevel defaults cap storage
                    // bindings at 128 MiB, far below what these cards allow.
                    required_limits: adapter.limits(),
                },
                None,
            )
            .await
            .map_err(|err| WgpuAccelError::DeviceRequestFailed(err.to_string()))?;

        let sha256 = ComputeKernel::new(&device, "x3-sha256", SHA256_WGSL, "main", DIGEST_WORDS);
        let keccak = ComputeKernel::new(&device, "x3-keccak", KECCAK_WGSL, "main", DIGEST_WORDS);

        Ok(Self {
            adapter_info,
            device,
            queue,
            sha256,
            keccak,
            secp256k1_kernels: OnceLock::new(),
        })
    }

    /// Stable backend label used by metrics.
    pub fn name(&self) -> &'static str {
        "wgpu"
    }

    /// Human-readable adapter name for diagnostics.
    pub fn adapter_name(&self) -> &str {
        &self.adapter_info.name
    }

    /// Full adapter record (backend, device type, driver) for bring-up
    /// diagnostics.
    ///
    /// `adapter_name` alone cannot distinguish "the real GPU is running the
    /// kernel" from "a software/fallback adapter answered the request", and a
    /// bring-up probe that cannot report *which* adapter executed is exactly
    /// how a CPU-only run gets mislabelled as GPU-accelerated. Exposing the
    /// whole record keeps that check in the caller's hands.
    pub fn adapter_info(&self) -> &wgpu::AdapterInfo {
        &self.adapter_info
    }

    /// SHA-256 of every input, in input order.
    pub fn sha256_batch(&self, inputs: &[Vec<u8>]) -> Result<Vec<[u8; 32]>, WgpuAccelError> {
        self.hash_batch(&self.sha256, inputs)
    }

    /// Keccak-256 (Ethereum's legacy `0x01`-domain padding, not SHA3-256) of
    /// every input, in input order.
    pub fn keccak256_batch(&self, inputs: &[Vec<u8>]) -> Result<Vec<[u8; 32]>, WgpuAccelError> {
        self.hash_batch(&self.keccak, inputs)
    }

    fn secp256k1_kernels(&self) -> &(ComputeKernel, ComputeKernel) {
        self.secp256k1_kernels.get_or_init(|| {
            let verify = ComputeKernel::new(
                &self.device,
                "x3-secp256k1",
                secp256k1::SECP256K1_WGSL,
                "main",
                1,
            );
            let selftest = ComputeKernel::new(
                &self.device,
                "x3-secp256k1-selftest",
                secp256k1::SECP256K1_WGSL,
                "selftest",
                24,
            );
            (verify, selftest)
        })
    }

    /// Split `inputs` into dispatches that fit the device and run them in order.
    fn hash_batch(
        &self,
        kernel: &ComputeKernel,
        inputs: &[Vec<u8>],
    ) -> Result<Vec<[u8; 32]>, WgpuAccelError> {
        let limits = self.device.limits();
        let max_bytes = MAX_DISPATCH_DATA_BYTES
            .min(u64::from(limits.max_storage_buffer_binding_size))
            .min(limits.max_buffer_size);
        // meta holds 2 words per message plus the header.
        let max_messages = ((max_bytes / 4) as usize / 2).saturating_sub(META_HEADER_WORDS);
        let mut outputs = Vec::with_capacity(inputs.len());
        let mut start = 0;
        while start < inputs.len() {
            let mut end = start;
            let mut bytes = 0u64;
            while end < inputs.len() && end - start < max_messages {
                if inputs[end].len() > MAX_MESSAGE_BYTES {
                    return Err(WgpuAccelError::InvalidInput(
                        "message exceeds u32 byte length",
                    ));
                }
                let padded = (inputs[end].len() as u64).div_ceil(4) * 4;
                if padded > max_bytes {
                    return Err(WgpuAccelError::InvalidInput(
                        "message larger than one GPU dispatch buffer",
                    ));
                }
                if end > start && bytes + padded > max_bytes {
                    break;
                }
                bytes += padded;
                end += 1;
            }
            outputs.extend(self.dispatch(kernel, &inputs[start..end], (bytes / 4) as usize)?);
            start = end;
        }
        Ok(outputs)
    }

    fn dispatch(
        &self,
        kernel: &ComputeKernel,
        inputs: &[Vec<u8>],
        data_words: usize,
    ) -> Result<Vec<[u8; 32]>, WgpuAccelError> {
        let bytes = self.execute(kernel, inputs.len(), data_words, |entries, data| {
            let mut byte = 0usize;
            for (index, input) in inputs.iter().enumerate() {
                let entry = 8 * index;
                entries[entry..entry + 4].copy_from_slice(&((byte / 4) as u32).to_le_bytes());
                entries[entry + 4..entry + 8].copy_from_slice(&(input.len() as u32).to_le_bytes());
                let padded = input.len().div_ceil(4) * 4;
                data[byte..byte + input.len()].copy_from_slice(input);
                // Staging memory is not zeroed; the kernels rely on these bytes being 0.
                data[byte + input.len()..byte + padded].fill(0);
                byte += padded;
            }
            data[byte..].fill(0);
        })?;
        // Both hash kernels write digest bytes in final order, so this is a copy.
        let (digests, _) = bytes.as_chunks::<32>();
        Ok(digests.to_vec())
    }

    /// Run `kernel` over `count` items in one dispatch and return its raw
    /// output (`count * kernel.output_words` words, little-endian).
    ///
    /// `fill(entries, data)` writes the per-item entries (2 words each, after
    /// the count header this function writes) and `data_words` words of input.
    /// Both slices are wgpu staging memory, which is not zeroed: `fill` must
    /// write every byte it is given.
    pub(crate) fn execute(
        &self,
        kernel: &ComputeKernel,
        count: usize,
        data_words: usize,
        fill: impl FnOnce(&mut [u8], &mut [u8]),
    ) -> Result<Vec<u8>, WgpuAccelError> {
        let count_u32 =
            u32::try_from(count).map_err(|_| WgpuAccelError::InvalidInput("batch exceeds u32"))?;
        // A batch of empty messages still needs a non-empty data binding.
        let data_words = data_words.max(1);
        let entry_words = META_HEADER_WORDS + 2 * count;
        let mut guard = kernel.buffers.lock().map_err(|_| {
            WgpuAccelError::BufferMapFailed(format!("{} buffer lock poisoned", kernel.label))
        })?;
        if !guard
            .as_ref()
            .is_some_and(|b| b.data_word_capacity >= data_words && b.message_capacity >= count)
        {
            *guard = None;
        }
        let buffers =
            &*guard.get_or_insert_with(|| kernel.create_buffers(&self.device, data_words, count));

        {
            let mut entries = self
                .queue
                .write_buffer_with(&buffers.meta_buffer, 0, non_zero_bytes(entry_words)?)
                .ok_or_else(|| WgpuAccelError::BufferMapFailed("entries staging".into()))?;
            entries[0..4].copy_from_slice(&count_u32.to_le_bytes());
            entries[4..8].fill(0);
            let mut data = self
                .queue
                .write_buffer_with(&buffers.data_buffer, 0, non_zero_bytes(data_words)?)
                .ok_or_else(|| WgpuAccelError::BufferMapFailed("data staging".into()))?;
            fill(&mut entries[META_HEADER_WORDS * 4..], &mut data);
        }

        let output_bytes = (count * kernel.output_words * 4) as wgpu::BufferAddress;
        if output_bytes == 0 {
            return Ok(Vec::new());
        }
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some(kernel.label),
            });
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some(kernel.label),
                timestamp_writes: None,
            });
            pass.set_pipeline(&kernel.pipeline);
            pass.set_bind_group(0, &buffers.bind_group, &[]);
            // 2-D grid: one dimension is capped at 65535 workgroups (~4.2M
            // items); the kernels fold y back into a linear index.
            let groups = count_u32.div_ceil(WORKGROUP_SIZE);
            let max_x = self.device.limits().max_compute_workgroups_per_dimension;
            let x = groups.min(max_x);
            pass.dispatch_workgroups(x, groups.div_ceil(x), 1);
        }
        encoder.copy_buffer_to_buffer(
            &buffers.output_buffer,
            0,
            &buffers.readback_buffer,
            0,
            output_bytes,
        );
        self.queue.submit(Some(encoder.finish()));

        let slice = buffers.readback_buffer.slice(0..output_bytes);
        let (sender, receiver) = mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |result| {
            let _ = sender.send(result);
        });
        self.device.poll(wgpu::Maintain::Wait);
        receiver
            .recv()
            .map_err(|err| WgpuAccelError::BufferMapFailed(err.to_string()))?
            .map_err(|err| WgpuAccelError::BufferMapFailed(err.to_string()))?;
        let bytes = slice.get_mapped_range().to_vec();
        buffers.readback_buffer.unmap();
        Ok(bytes)
    }
}

fn non_zero_bytes(words: usize) -> Result<NonZeroU64, WgpuAccelError> {
    NonZeroU64::new((words * 4) as u64).ok_or(WgpuAccelError::InvalidInput(
        "a staging write must cover at least one word",
    ))
}

impl ComputeKernel {
    pub(crate) fn new(
        device: &wgpu::Device,
        label: &'static str,
        source: &str,
        entry_point: &str,
        output_words: usize,
    ) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some(label),
            source: wgpu::ShaderSource::Wgsl(source.into()),
        });
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some(label),
            entries: &[
                storage_entry(0, true),
                storage_entry(1, true),
                storage_entry(2, false),
            ],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some(label),
            bind_group_layouts: &[&layout],
            push_constant_ranges: &[],
        });
        let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some(label),
            layout: Some(&pipeline_layout),
            module: &shader,
            entry_point,
            compilation_options: Default::default(),
        });
        Self {
            label,
            output_words,
            layout,
            pipeline,
            buffers: Mutex::new(None),
        }
    }

    /// Buffers grow to the next power of two and are then reused, so a steady
    /// stream of similar batches allocates nothing after warm-up.
    fn create_buffers(
        &self,
        device: &wgpu::Device,
        data_words: usize,
        messages: usize,
    ) -> HashBuffers {
        // `hash_batch` already bounds each dispatch by the device limit; growth
        // is capped there too so a power-of-two round-up never exceeds it.
        let limit_words = (device.limits().max_storage_buffer_binding_size / 4) as usize;
        let data_word_capacity = data_words
            .next_power_of_two()
            .min(limit_words)
            .max(data_words);
        let message_limit = limit_words.saturating_sub(META_HEADER_WORDS) / 2;
        let message_capacity = messages
            .next_power_of_two()
            .min(message_limit)
            .max(messages);
        let meta_words = META_HEADER_WORDS + 2 * message_capacity;
        let buffer = |name: &str, words: usize, usage: wgpu::BufferUsages| {
            device.create_buffer(&wgpu::BufferDescriptor {
                label: Some(&format!("{}-{name}", self.label)),
                size: (words * 4) as wgpu::BufferAddress,
                usage,
                mapped_at_creation: false,
            })
        };
        let storage_in = wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST;
        let data_buffer = buffer("data", data_word_capacity, storage_in);
        let meta_buffer = buffer("meta", meta_words, storage_in);
        let output_buffer = buffer(
            "output",
            message_capacity * self.output_words,
            wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        );
        let readback_buffer = buffer(
            "readback",
            message_capacity * self.output_words,
            wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        );
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some(self.label),
            layout: &self.layout,
            entries: &[
                bind_entry(0, &data_buffer),
                bind_entry(1, &meta_buffer),
                bind_entry(2, &output_buffer),
            ],
        });
        HashBuffers {
            data_word_capacity,
            message_capacity,
            data_buffer,
            meta_buffer,
            output_buffer,
            readback_buffer,
            bind_group,
        }
    }
}

fn instance() -> wgpu::Instance {
    wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::VULKAN
            | wgpu::Backends::METAL
            | wgpu::Backends::DX12
            | wgpu::Backends::GL,
        ..Default::default()
    })
}

fn hardware_adapters_raw() -> Vec<wgpu::Adapter> {
    let mut adapters: Vec<wgpu::Adapter> = instance()
        .enumerate_adapters(wgpu::Backends::VULKAN | wgpu::Backends::METAL | wgpu::Backends::DX12)
        .into_iter()
        .filter(|adapter| {
            matches!(
                adapter.get_info().device_type,
                wgpu::DeviceType::DiscreteGpu | wgpu::DeviceType::IntegratedGpu
            )
        })
        .collect();
    adapters.sort_by_key(|adapter| {
        let info = adapter.get_info();
        (info.vendor, info.device, info.name)
    });
    // One entry per physical device even if several backends expose it.
    adapters.dedup_by_key(|adapter| {
        let info = adapter.get_info();
        (info.vendor, info.device, info.name)
    });
    adapters
}

/// Return true when a hardware wgpu adapter/device can be initialized.
pub fn is_available() -> bool {
    WgpuBackend::initialize().is_ok()
}

fn storage_entry(binding: u32, read_only: bool) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::COMPUTE,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Storage { read_only },
            has_dynamic_offset: false,
            min_binding_size: None,
        },
        count: None,
    }
}

fn bind_entry(binding: u32, buffer: &wgpu::Buffer) -> wgpu::BindGroupEntry<'_> {
    wgpu::BindGroupEntry {
        binding,
        resource: buffer.as_entire_binding(),
    }
}

const SHA256_WGSL: &str = r#"
@group(0) @binding(0) var<storage, read> data: array<u32>;
@group(0) @binding(1) var<storage, read> entries: array<u32>;
@group(0) @binding(2) var<storage, read_write> output_words: array<u32>;

fn bswap(x: u32) -> u32 {
    return (x << 24u) | ((x << 8u) & 0x00ff0000u) | ((x >> 8u) & 0x0000ff00u) | (x >> 24u);
}

// Big-endian word at byte `p` of the padded message: data, then 0x80, then
// zeros, then the 64-bit big-endian bit length in the last 8 bytes. The host
// zero-fills each message to a word boundary, so a partial data word needs no
// masking.
fn padded_word(base: u32, len: u32, p: u32, total: u32) -> u32 {
    if (p == total - 8u) {
        return len >> 29u;
    }
    if (p == total - 4u) {
        return len << 3u;
    }
    var w = 0u;
    if (p < len) {
        w = bswap(data[base + (p >> 2u)]);
    }
    if (len >= p && len < p + 4u) {
        w = w | (0x80u << ((3u - (len - p)) * 8u));
    }
    return w;
}

fn k(t: u32) -> u32 {
    switch (t) {
        case 0u: { return 0x428a2f98u; }
        case 1u: { return 0x71374491u; }
        case 2u: { return 0xb5c0fbcfu; }
        case 3u: { return 0xe9b5dba5u; }
        case 4u: { return 0x3956c25bu; }
        case 5u: { return 0x59f111f1u; }
        case 6u: { return 0x923f82a4u; }
        case 7u: { return 0xab1c5ed5u; }
        case 8u: { return 0xd807aa98u; }
        case 9u: { return 0x12835b01u; }
        case 10u: { return 0x243185beu; }
        case 11u: { return 0x550c7dc3u; }
        case 12u: { return 0x72be5d74u; }
        case 13u: { return 0x80deb1feu; }
        case 14u: { return 0x9bdc06a7u; }
        case 15u: { return 0xc19bf174u; }
        case 16u: { return 0xe49b69c1u; }
        case 17u: { return 0xefbe4786u; }
        case 18u: { return 0x0fc19dc6u; }
        case 19u: { return 0x240ca1ccu; }
        case 20u: { return 0x2de92c6fu; }
        case 21u: { return 0x4a7484aau; }
        case 22u: { return 0x5cb0a9dcu; }
        case 23u: { return 0x76f988dau; }
        case 24u: { return 0x983e5152u; }
        case 25u: { return 0xa831c66du; }
        case 26u: { return 0xb00327c8u; }
        case 27u: { return 0xbf597fc7u; }
        case 28u: { return 0xc6e00bf3u; }
        case 29u: { return 0xd5a79147u; }
        case 30u: { return 0x06ca6351u; }
        case 31u: { return 0x14292967u; }
        case 32u: { return 0x27b70a85u; }
        case 33u: { return 0x2e1b2138u; }
        case 34u: { return 0x4d2c6dfcu; }
        case 35u: { return 0x53380d13u; }
        case 36u: { return 0x650a7354u; }
        case 37u: { return 0x766a0abbu; }
        case 38u: { return 0x81c2c92eu; }
        case 39u: { return 0x92722c85u; }
        case 40u: { return 0xa2bfe8a1u; }
        case 41u: { return 0xa81a664bu; }
        case 42u: { return 0xc24b8b70u; }
        case 43u: { return 0xc76c51a3u; }
        case 44u: { return 0xd192e819u; }
        case 45u: { return 0xd6990624u; }
        case 46u: { return 0xf40e3585u; }
        case 47u: { return 0x106aa070u; }
        case 48u: { return 0x19a4c116u; }
        case 49u: { return 0x1e376c08u; }
        case 50u: { return 0x2748774cu; }
        case 51u: { return 0x34b0bcb5u; }
        case 52u: { return 0x391c0cb3u; }
        case 53u: { return 0x4ed8aa4au; }
        case 54u: { return 0x5b9cca4fu; }
        case 55u: { return 0x682e6ff3u; }
        case 56u: { return 0x748f82eeu; }
        case 57u: { return 0x78a5636fu; }
        case 58u: { return 0x84c87814u; }
        case 59u: { return 0x8cc70208u; }
        case 60u: { return 0x90befffau; }
        case 61u: { return 0xa4506cebu; }
        case 62u: { return 0xbef9a3f7u; }
        default: { return 0xc67178f2u; }
    }
}

fn rotr(x: u32, n: u32) -> u32 {
    return (x >> n) | (x << (32u - n));
}

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>, @builtin(num_workgroups) groups: vec3<u32>) {
    let idx = global_id.x + global_id.y * groups.x * 64u;
    if (idx >= entries[0]) {
        return;
    }

    let base = entries[2u + 2u * idx];
    let len = entries[3u + 2u * idx];
    // 1 byte of 0x80 and 8 bytes of length must fit after the data.
    let count = (len + 8u) / 64u + 1u;
    let total = count * 64u;

    var h0 = 0x6a09e667u;
    var h1 = 0xbb67ae85u;
    var h2 = 0x3c6ef372u;
    var h3 = 0xa54ff53au;
    var h4 = 0x510e527fu;
    var h5 = 0x9b05688cu;
    var h6 = 0x1f83d9abu;
    var h7 = 0x5be0cd19u;

    for (var block = 0u; block < count; block = block + 1u) {
        var w: array<u32, 64>;
        let block_base = block * 64u;

        for (var t = 0u; t < 16u; t = t + 1u) {
            w[t] = padded_word(base, len, block_base + 4u * t, total);
        }

        for (var t = 16u; t < 64u; t = t + 1u) {
            let s0 = rotr(w[t - 15u], 7u) ^ rotr(w[t - 15u], 18u) ^ (w[t - 15u] >> 3u);
            let s1 = rotr(w[t - 2u], 17u) ^ rotr(w[t - 2u], 19u) ^ (w[t - 2u] >> 10u);
            w[t] = w[t - 16u] + s0 + w[t - 7u] + s1;
        }

        var a = h0;
        var b = h1;
        var c = h2;
        var d = h3;
        var e = h4;
        var f = h5;
        var g = h6;
        var h = h7;

        for (var t = 0u; t < 64u; t = t + 1u) {
            let s1 = rotr(e, 6u) ^ rotr(e, 11u) ^ rotr(e, 25u);
            let ch = (e & f) ^ ((~e) & g);
            let temp1 = h + s1 + ch + k(t) + w[t];
            let s0 = rotr(a, 2u) ^ rotr(a, 13u) ^ rotr(a, 22u);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let temp2 = s0 + maj;
            h = g;
            g = f;
            f = e;
            e = d + temp1;
            d = c;
            c = b;
            b = a;
            a = temp1 + temp2;
        }

        h0 = h0 + a;
        h1 = h1 + b;
        h2 = h2 + c;
        h3 = h3 + d;
        h4 = h4 + e;
        h5 = h5 + f;
        h6 = h6 + g;
        h7 = h7 + h;
    }

    // Byte-swapped so the buffer holds the digest bytes in order.
    let out = idx * 8u;
    output_words[out] = bswap(h0);
    output_words[out + 1u] = bswap(h1);
    output_words[out + 2u] = bswap(h2);
    output_words[out + 3u] = bswap(h3);
    output_words[out + 4u] = bswap(h4);
    output_words[out + 5u] = bswap(h5);
    output_words[out + 6u] = bswap(h6);
    output_words[out + 7u] = bswap(h7);
}
"#;

/// Keccak-f[1600] on two 32-bit halves per lane.
///
/// WGSL only exposes 64-bit integers behind `Features::SHADER_INT64`, and that
/// feature is optional on the adapters this backend targets (it is absent on
/// older drivers), so a kernel written with `u64` would fail pipeline creation
/// on exactly the machines that need the accelerator. Splitting each lane into
/// a low/high `u32` pair keeps the whole permutation inside baseline WGSL: the
/// only operation that needs care is the 64-bit rotate, which is assembled from
/// two 32-bit shifts in `rotl_lane`.
const KECCAK_WGSL: &str = r#"
@group(0) @binding(0) var<storage, read> data: array<u32>;
@group(0) @binding(1) var<storage, read> entries: array<u32>;
@group(0) @binding(2) var<storage, read_write> output_words: array<u32>;

// Little-endian word at byte `p` of the pad10*1-padded message: data, then the
// 0x01 domain byte, zeros, and 0x80 OR-ed into the last byte of the last block
// (both land on one byte, 0x81, when len % 136 == 135). The host zero-fills
// each message to a word boundary, so a partial data word needs no masking.
fn padded_word(base: u32, len: u32, p: u32, last_byte: u32) -> u32 {
    var w = 0u;
    if (p < len) {
        w = data[base + (p >> 2u)];
    }
    if (len >= p && len < p + 4u) {
        w = w | (0x01u << ((len - p) * 8u));
    }
    if (last_byte >= p && last_byte < p + 4u) {
        w = w | (0x80u << ((last_byte - p) * 8u));
    }
    return w;
}

fn rc_lo(index: u32) -> u32 {
    switch index {
        case 0u: { return 0x00000001u; }
        case 1u: { return 0x00008082u; }
        case 2u: { return 0x0000808au; }
        case 3u: { return 0x80008000u; }
        case 4u: { return 0x0000808bu; }
        case 5u: { return 0x80000001u; }
        case 6u: { return 0x80008081u; }
        case 7u: { return 0x00008009u; }
        case 8u: { return 0x0000008au; }
        case 9u: { return 0x00000088u; }
        case 10u: { return 0x80008009u; }
        case 11u: { return 0x8000000au; }
        case 12u: { return 0x8000808bu; }
        case 13u: { return 0x0000008bu; }
        case 14u: { return 0x00008089u; }
        case 15u: { return 0x00008003u; }
        case 16u: { return 0x00008002u; }
        case 17u: { return 0x00000080u; }
        case 18u: { return 0x0000800au; }
        case 19u: { return 0x8000000au; }
        case 20u: { return 0x80008081u; }
        case 21u: { return 0x00008080u; }
        case 22u: { return 0x80000001u; }
        case 23u: { return 0x80008008u; }
        default: { return 0u; }
    }
}

fn rc_hi(index: u32) -> u32 {
    switch index {
        case 0u: { return 0x00000000u; }
        case 1u: { return 0x00000000u; }
        case 2u: { return 0x80000000u; }
        case 3u: { return 0x80000000u; }
        case 4u: { return 0x00000000u; }
        case 5u: { return 0x00000000u; }
        case 6u: { return 0x80000000u; }
        case 7u: { return 0x80000000u; }
        case 8u: { return 0x00000000u; }
        case 9u: { return 0x00000000u; }
        case 10u: { return 0x00000000u; }
        case 11u: { return 0x00000000u; }
        case 12u: { return 0x00000000u; }
        case 13u: { return 0x80000000u; }
        case 14u: { return 0x80000000u; }
        case 15u: { return 0x80000000u; }
        case 16u: { return 0x80000000u; }
        case 17u: { return 0x80000000u; }
        case 18u: { return 0x00000000u; }
        case 19u: { return 0x80000000u; }
        case 20u: { return 0x80000000u; }
        case 21u: { return 0x80000000u; }
        case 22u: { return 0x00000000u; }
        case 23u: { return 0x80000000u; }
        default: { return 0u; }
    }
}

fn rho_offset(index: u32) -> u32 {
    switch index {
        case 0u: { return 0u; }
        case 1u: { return 1u; }
        case 2u: { return 62u; }
        case 3u: { return 28u; }
        case 4u: { return 27u; }
        case 5u: { return 36u; }
        case 6u: { return 44u; }
        case 7u: { return 6u; }
        case 8u: { return 55u; }
        case 9u: { return 20u; }
        case 10u: { return 3u; }
        case 11u: { return 10u; }
        case 12u: { return 43u; }
        case 13u: { return 25u; }
        case 14u: { return 39u; }
        case 15u: { return 41u; }
        case 16u: { return 45u; }
        case 17u: { return 15u; }
        case 18u: { return 21u; }
        case 19u: { return 8u; }
        case 20u: { return 18u; }
        case 21u: { return 2u; }
        case 22u: { return 61u; }
        case 23u: { return 56u; }
        case 24u: { return 14u; }
        default: { return 0u; }
    }
}

fn pi_dest(index: u32) -> u32 {
    switch index {
        case 0u: { return 0u; }
        case 1u: { return 10u; }
        case 2u: { return 20u; }
        case 3u: { return 5u; }
        case 4u: { return 15u; }
        case 5u: { return 16u; }
        case 6u: { return 1u; }
        case 7u: { return 11u; }
        case 8u: { return 21u; }
        case 9u: { return 6u; }
        case 10u: { return 7u; }
        case 11u: { return 17u; }
        case 12u: { return 2u; }
        case 13u: { return 12u; }
        case 14u: { return 22u; }
        case 15u: { return 23u; }
        case 16u: { return 8u; }
        case 17u: { return 18u; }
        case 18u: { return 3u; }
        case 19u: { return 13u; }
        case 20u: { return 14u; }
        case 21u: { return 24u; }
        case 22u: { return 9u; }
        case 23u: { return 19u; }
        case 24u: { return 4u; }
        default: { return 0u; }
    }
}

fn rotl_lane(lo: u32, hi: u32, amount: u32) -> vec2<u32> {
    if (amount == 0u) {
        return vec2<u32>(lo, hi);
    }
    if (amount == 32u) {
        return vec2<u32>(hi, lo);
    }
    if (amount < 32u) {
        let back = 32u - amount;
        return vec2<u32>((lo << amount) | (hi >> back), (hi << amount) | (lo >> back));
    }
    let forward = amount - 32u;
    let back = 32u - forward;
    return vec2<u32>((hi << forward) | (lo >> back), (lo << forward) | (hi >> back));
}

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) global_id: vec3<u32>, @builtin(num_workgroups) groups: vec3<u32>) {
    let idx = global_id.x + global_id.y * groups.x * 64u;
    if (idx >= entries[0]) {
        return;
    }

    let base = entries[2u + 2u * idx];
    let len = entries[3u + 2u * idx];
    // The 0x01 domain byte always fits, so there is one block past the data.
    let count = len / 136u + 1u;
    let last_byte = count * 136u - 1u;

    var lo: array<u32, 25>;
    var hi: array<u32, 25>;
    for (var lane = 0u; lane < 25u; lane = lane + 1u) {
        lo[lane] = 0u;
        hi[lane] = 0u;
    }

    for (var block = 0u; block < count; block = block + 1u) {
        let block_base = block * 136u;
        for (var lane = 0u; lane < 17u; lane = lane + 1u) {
            let p = block_base + 8u * lane;
            lo[lane] = lo[lane] ^ padded_word(base, len, p, last_byte);
            hi[lane] = hi[lane] ^ padded_word(base, len, p + 4u, last_byte);
        }

        for (var round = 0u; round < 24u; round = round + 1u) {
            var column: array<vec2<u32>, 5>;
            for (var x = 0u; x < 5u; x = x + 1u) {
                column[x] = vec2<u32>(
                    lo[x] ^ lo[x + 5u] ^ lo[x + 10u] ^ lo[x + 15u] ^ lo[x + 20u],
                    hi[x] ^ hi[x + 5u] ^ hi[x + 10u] ^ hi[x + 15u] ^ hi[x + 20u],
                );
            }

            var diff: array<vec2<u32>, 5>;
            for (var x = 0u; x < 5u; x = x + 1u) {
                let previous = column[(x + 4u) % 5u];
                let next_column = column[(x + 1u) % 5u];
                let rotated = rotl_lane(next_column.x, next_column.y, 1u);
                diff[x] = vec2<u32>(previous.x ^ rotated.x, previous.y ^ rotated.y);
            }
            for (var y = 0u; y < 5u; y = y + 1u) {
                for (var x = 0u; x < 5u; x = x + 1u) {
                    let lane = x + 5u * y;
                    lo[lane] = lo[lane] ^ diff[x].x;
                    hi[lane] = hi[lane] ^ diff[x].y;
                }
            }

            var mixed_lo: array<u32, 25>;
            var mixed_hi: array<u32, 25>;
            for (var lane = 0u; lane < 25u; lane = lane + 1u) {
                let rotated = rotl_lane(lo[lane], hi[lane], rho_offset(lane));
                let destination = pi_dest(lane);
                mixed_lo[destination] = rotated.x;
                mixed_hi[destination] = rotated.y;
            }

            for (var y = 0u; y < 5u; y = y + 1u) {
                for (var x = 0u; x < 5u; x = x + 1u) {
                    let lane = x + 5u * y;
                    let next = ((x + 1u) % 5u) + 5u * y;
                    let after_next = ((x + 2u) % 5u) + 5u * y;
                    lo[lane] = mixed_lo[lane] ^ ((~mixed_lo[next]) & mixed_lo[after_next]);
                    hi[lane] = mixed_hi[lane] ^ ((~mixed_hi[next]) & mixed_hi[after_next]);
                }
            }

            lo[0u] = lo[0u] ^ rc_lo(round);
            hi[0u] = hi[0u] ^ rc_hi(round);
        }
    }

    let out = idx * 8u;
    output_words[out] = lo[0u];
    output_words[out + 1u] = hi[0u];
    output_words[out + 2u] = lo[1u];
    output_words[out + 3u] = hi[1u];
    output_words[out + 4u] = lo[2u];
    output_words[out + 5u] = hi[2u];
    output_words[out + 6u] = lo[3u];
    output_words[out + 7u] = hi[3u];
}
"#;

#[cfg(test)]
mod tests {
    use super::*;
    use sha2::{Digest, Sha256};

    /// The GPU backend, or `None` with a visible skip. `X3_REQUIRE_GPU=1`
    /// turns a missing GPU into a failure so a hardware gate cannot pass
    /// without having touched a GPU.
    fn gpu_or_skip(test: &str) -> Option<WgpuBackend> {
        match WgpuBackend::initialize() {
            Ok(backend) => {
                eprintln!("{test}: running on {}", backend.adapter_name());
                Some(backend)
            }
            Err(err) => {
                assert!(
                    std::env::var("X3_REQUIRE_GPU").as_deref() != Ok("1"),
                    "{test}: X3_REQUIRE_GPU=1 but no GPU adapter: {err}"
                );
                eprintln!("{test}: SKIPPED, no GPU adapter ({err})");
                None
            }
        }
    }

    #[test]
    fn sha256_kernel_matches_cpu_when_wgpu_is_available() {
        let Some(backend) = gpu_or_skip("sha256_kernel_matches_cpu") else {
            return;
        };

        let inputs = vec![b"abc".to_vec(), Vec::new(), b"x3".to_vec(), vec![42u8; 120]];
        let gpu_outputs = backend.sha256_batch(&inputs).unwrap();
        let cpu_outputs = inputs
            .iter()
            .map(|input| Sha256::digest(input).into())
            .collect::<Vec<[u8; 32]>>();

        assert_eq!(gpu_outputs, cpu_outputs);
    }

    /// Every length 0..=600 plus the multi-block edges: the kernels pad on the
    /// GPU, so each block-boundary case for both algorithms is a parity case.
    fn boundary_inputs() -> Vec<Vec<u8>> {
        let mut inputs: Vec<Vec<u8>> = (0..=600usize)
            .map(|len| (0..len).map(|i| (i * 31 + len) as u8).collect())
            .collect();
        for len in [1023, 1024, 1025, 4095, 4096, 4097, 65_536] {
            inputs.push(vec![0xa5; len]);
        }
        // A byte that would leak into the padding if the host stopped
        // zero-filling the word tail.
        inputs.push(vec![0xff; 3]);
        inputs
    }

    #[test]
    fn every_padding_boundary_matches_cpu() {
        let Some(backend) = gpu_or_skip("every_padding_boundary_matches_cpu") else {
            return;
        };
        let inputs = boundary_inputs();
        let keccak = backend.keccak256_batch(&inputs).unwrap();
        let sha = backend.sha256_batch(&inputs).unwrap();
        for (i, input) in inputs.iter().enumerate() {
            assert_eq!(
                keccak[i],
                keccak_hash::keccak(input).0,
                "keccak len {}",
                input.len()
            );
            let expected: [u8; 32] = Sha256::digest(input).into();
            assert_eq!(sha[i], expected, "sha256 len {}", input.len());
        }
    }

    #[test]
    fn reused_buffers_do_not_leak_previous_batches() {
        let Some(backend) = gpu_or_skip("reused_buffers_do_not_leak_previous_batches") else {
            return;
        };
        // A large batch first fills the reusable buffers with nonzero bytes;
        // a smaller one afterwards must not see any of them.
        backend
            .keccak256_batch(&vec![vec![0xff; 999]; 2048])
            .unwrap();
        backend.sha256_batch(&vec![vec![0xff; 999]; 2048]).unwrap();
        let small = vec![Vec::new(), vec![1u8], vec![2u8; 5], vec![3u8; 137]];
        let keccak = backend.keccak256_batch(&small).unwrap();
        let sha = backend.sha256_batch(&small).unwrap();
        for (i, input) in small.iter().enumerate() {
            assert_eq!(keccak[i], keccak_hash::keccak(input).0);
            let expected: [u8; 32] = Sha256::digest(input).into();
            assert_eq!(sha[i], expected);
        }
    }

    #[test]
    fn all_empty_batch_and_empty_list() {
        let Some(backend) = gpu_or_skip("all_empty_batch_and_empty_list") else {
            return;
        };
        assert!(backend.keccak256_batch(&[]).unwrap().is_empty());
        let empties = vec![Vec::new(); 70];
        assert_eq!(
            backend.keccak256_batch(&empties).unwrap(),
            vec![keccak_hash::keccak([]).0; 70]
        );
        let expected: [u8; 32] = Sha256::digest([]).into();
        assert_eq!(backend.sha256_batch(&empties).unwrap(), vec![expected; 70]);
    }

    #[test]
    fn keccak256_kernel_matches_cpu_when_wgpu_is_available() {
        let Some(backend) = gpu_or_skip("keccak256_kernel_matches_cpu") else {
            return;
        };

        // The 135/136/137-byte messages straddle Keccak's 136-byte absorb rate,
        // which is where a kernel that absorbs the wrong number of lanes
        // diverges from `keccak-hash`.
        let inputs = vec![
            Vec::new(),
            b"abc".to_vec(),
            vec![0xabu8; 135],
            vec![0x11u8; 136],
            vec![0x22u8; 137],
            vec![0x33u8; 1024],
        ];
        let gpu_outputs = backend.keccak256_batch(&inputs).unwrap();
        let cpu_outputs = inputs
            .iter()
            .map(|input| keccak_hash::keccak(input).0)
            .collect::<Vec<[u8; 32]>>();

        assert_eq!(gpu_outputs, cpu_outputs);
    }
}
