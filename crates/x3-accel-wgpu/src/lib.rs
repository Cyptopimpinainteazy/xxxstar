//! wgpu support crate for X3 accelerator backends.
//!
//! This crate owns device/adapter discovery for non-CUDA acceleration. Kernel
//! implementations are deliberately explicit: algorithms without a WGSL kernel
//! return an error instead of silently falling back inside the backend.

use std::sync::{mpsc, Mutex, MutexGuard};

const SHA256_WORDS: usize = 8;
const SHA256_BLOCK_WORDS: usize = 16;
const WORKGROUP_SIZE: u32 = 64;

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

/// Minimal wgpu backend handle with cached SHA256 compute resources.
pub struct WgpuBackend {
    adapter_info: wgpu::AdapterInfo,
    device: wgpu::Device,
    queue: wgpu::Queue,
    sha256_bind_group_layout: wgpu::BindGroupLayout,
    sha256_pipeline: wgpu::ComputePipeline,
    sha256_buffers: Mutex<Option<Sha256Buffers>>,
}

struct Sha256Buffers {
    block_word_capacity: usize,
    message_capacity: usize,
    output_word_capacity: usize,
    blocks_buffer: wgpu::Buffer,
    block_offsets_buffer: wgpu::Buffer,
    block_counts_buffer: wgpu::Buffer,
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
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::VULKAN
                | wgpu::Backends::METAL
                | wgpu::Backends::DX12
                | wgpu::Backends::GL,
            ..Default::default()
        });

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: None,
                force_fallback_adapter: false,
            })
            .await
            .ok_or(WgpuAccelError::AdapterUnavailable)?;

        let adapter_info = adapter.get_info();
        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("x3-accel-wgpu"),
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::downlevel_defaults(),
                },
                None,
            )
            .await
            .map_err(|err| WgpuAccelError::DeviceRequestFailed(err.to_string()))?;

        let (sha256_bind_group_layout, sha256_pipeline) = create_sha256_pipeline(&device);

        Ok(Self {
            adapter_info,
            device,
            queue,
            sha256_bind_group_layout,
            sha256_pipeline,
            sha256_buffers: Mutex::new(None),
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

    /// SHA256 compute kernel entrypoint.
    pub fn sha256_batch(&self, inputs: &[Vec<u8>]) -> Result<Vec<[u8; 32]>, WgpuAccelError> {
        if inputs.is_empty() {
            return Ok(Vec::new());
        }

        let mut block_words = Vec::new();
        let mut block_offsets = Vec::with_capacity(inputs.len());
        let mut block_counts = Vec::with_capacity(inputs.len());
        for input in inputs {
            block_offsets.push(
                u32::try_from(block_words.len() / SHA256_BLOCK_WORDS)
                    .map_err(|_| WgpuAccelError::InvalidInput("block offset exceeds u32"))?,
            );
            let before = block_words.len();
            append_padded_sha256_blocks(input, &mut block_words)?;
            block_counts.push(
                u32::try_from((block_words.len() - before) / SHA256_BLOCK_WORDS)
                    .map_err(|_| WgpuAccelError::InvalidInput("block count exceeds u32"))?,
            );
        }

        let output_words = inputs.len() * SHA256_WORDS;
        let output_size = (output_words * std::mem::size_of::<u32>()) as wgpu::BufferAddress;
        let buffers_guard = self.sha256_buffers(block_words.len(), inputs.len(), output_words)?;
        let buffers = buffers_guard
            .as_ref()
            .expect("sha256 buffers initialized after capacity check");

        self.queue.write_buffer(
            &buffers.blocks_buffer,
            0,
            bytemuck::cast_slice(&block_words),
        );
        self.queue.write_buffer(
            &buffers.block_offsets_buffer,
            0,
            bytemuck::cast_slice(&block_offsets),
        );
        self.queue.write_buffer(
            &buffers.block_counts_buffer,
            0,
            bytemuck::cast_slice(&block_counts),
        );

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("x3-sha256-encoder"),
            });
        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("x3-sha256-compute-pass"),
                timestamp_writes: None,
            });
            compute_pass.set_pipeline(&self.sha256_pipeline);
            compute_pass.set_bind_group(0, &buffers.bind_group, &[]);
            let workgroups = (inputs.len() as u32).div_ceil(WORKGROUP_SIZE);
            compute_pass.dispatch_workgroups(workgroups, 1, 1);
        }
        encoder.copy_buffer_to_buffer(
            &buffers.output_buffer,
            0,
            &buffers.readback_buffer,
            0,
            output_size,
        );
        self.queue.submit(Some(encoder.finish()));

        let slice = buffers.readback_buffer.slice(0..output_size);
        let (sender, receiver) = mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |result| {
            let _ = sender.send(result);
        });
        self.device.poll(wgpu::Maintain::Wait);
        receiver
            .recv()
            .map_err(|err| WgpuAccelError::BufferMapFailed(err.to_string()))?
            .map_err(|err| WgpuAccelError::BufferMapFailed(err.to_string()))?;

        let mapped = slice.get_mapped_range();
        let words = bytemuck::cast_slice::<u8, u32>(&mapped).to_vec();
        drop(mapped);
        buffers.readback_buffer.unmap();

        let mut outputs = Vec::with_capacity(inputs.len());
        for chunk in words.chunks_exact(SHA256_WORDS) {
            let mut output = [0u8; 32];
            for (index, word) in chunk.iter().enumerate() {
                output[index * 4..index * 4 + 4].copy_from_slice(&word.to_be_bytes());
            }
            outputs.push(output);
        }

        Ok(outputs)
    }

    fn sha256_buffers(
        &self,
        required_block_words: usize,
        required_messages: usize,
        required_output_words: usize,
    ) -> Result<MutexGuard<'_, Option<Sha256Buffers>>, WgpuAccelError> {
        let mut guard = self
            .sha256_buffers
            .lock()
            .map_err(|_| WgpuAccelError::BufferMapFailed("sha256 buffer lock poisoned".into()))?;

        let needs_create = guard
            .as_ref()
            .map(|buffers| {
                buffers.block_word_capacity < required_block_words
                    || buffers.message_capacity < required_messages
                    || buffers.output_word_capacity < required_output_words
            })
            .unwrap_or(true);

        if needs_create {
            let block_word_capacity = required_block_words.max(16).next_power_of_two();
            let message_capacity = required_messages.max(1).next_power_of_two();
            let output_word_capacity = required_output_words.max(8).next_power_of_two();

            *guard = Some(create_sha256_buffers(
                &self.device,
                &self.sha256_bind_group_layout,
                block_word_capacity,
                message_capacity,
                output_word_capacity,
            ));
        }

        Ok(guard)
    }
}

/// Return true when a wgpu adapter/device can be initialized.
pub fn is_available() -> bool {
    WgpuBackend::initialize().is_ok()
}

fn append_padded_sha256_blocks(
    input: &[u8],
    output_words: &mut Vec<u32>,
) -> Result<(), WgpuAccelError> {
    let bit_len = u64::try_from(input.len())
        .map_err(|_| WgpuAccelError::InvalidInput("input length exceeds u64"))?
        .checked_mul(8)
        .ok_or(WgpuAccelError::InvalidInput("input bit length overflow"))?;

    let mut padded = Vec::with_capacity(input.len() + 1 + 8 + 64);
    padded.extend_from_slice(input);
    padded.push(0x80);
    while padded.len() % 64 != 56 {
        padded.push(0);
    }
    padded.extend_from_slice(&bit_len.to_be_bytes());

    if padded.len() / 64 > u32::MAX as usize {
        return Err(WgpuAccelError::InvalidInput(
            "padded input block count exceeds u32",
        ));
    }

    for block in padded.chunks_exact(64) {
        for word in block.chunks_exact(4) {
            output_words.push(u32::from_be_bytes([word[0], word[1], word[2], word[3]]));
        }
    }

    Ok(())
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

fn create_sha256_pipeline(device: &wgpu::Device) -> (wgpu::BindGroupLayout, wgpu::ComputePipeline) {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("x3-sha256-wgsl"),
        source: wgpu::ShaderSource::Wgsl(SHA256_WGSL.into()),
    });
    let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("x3-sha256-bind-group-layout"),
        entries: &[
            storage_entry(0, true),
            storage_entry(1, true),
            storage_entry(2, true),
            storage_entry(3, false),
        ],
    });
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("x3-sha256-pipeline-layout"),
        bind_group_layouts: &[&bind_group_layout],
        push_constant_ranges: &[],
    });
    let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some("x3-sha256-pipeline"),
        layout: Some(&pipeline_layout),
        module: &shader,
        entry_point: "main",
        compilation_options: Default::default(),
    });

    (bind_group_layout, pipeline)
}

fn create_sha256_buffers(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    block_word_capacity: usize,
    message_capacity: usize,
    output_word_capacity: usize,
) -> Sha256Buffers {
    let blocks_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("x3-sha256-blocks"),
        size: bytes_for_words(block_word_capacity),
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let block_offsets_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("x3-sha256-block-offsets"),
        size: bytes_for_words(message_capacity),
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let block_counts_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("x3-sha256-block-counts"),
        size: bytes_for_words(message_capacity),
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let output_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("x3-sha256-output"),
        size: bytes_for_words(output_word_capacity),
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    });
    let readback_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("x3-sha256-readback"),
        size: bytes_for_words(output_word_capacity),
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("x3-sha256-bind-group"),
        layout,
        entries: &[
            bind_entry(0, &blocks_buffer),
            bind_entry(1, &block_offsets_buffer),
            bind_entry(2, &block_counts_buffer),
            bind_entry(3, &output_buffer),
        ],
    });

    Sha256Buffers {
        block_word_capacity,
        message_capacity,
        output_word_capacity,
        blocks_buffer,
        block_offsets_buffer,
        block_counts_buffer,
        output_buffer,
        readback_buffer,
        bind_group,
    }
}

fn bytes_for_words(words: usize) -> wgpu::BufferAddress {
    (words * std::mem::size_of::<u32>()) as wgpu::BufferAddress
}

fn bind_entry(binding: u32, buffer: &wgpu::Buffer) -> wgpu::BindGroupEntry<'_> {
    wgpu::BindGroupEntry {
        binding,
        resource: buffer.as_entire_binding(),
    }
}

const SHA256_WGSL: &str = r#"
@group(0) @binding(0) var<storage, read> block_words: array<u32>;
@group(0) @binding(1) var<storage, read> block_offsets: array<u32>;
@group(0) @binding(2) var<storage, read> block_counts: array<u32>;
@group(0) @binding(3) var<storage, read_write> output_words: array<u32>;

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
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let idx = global_id.x;
    if (idx >= arrayLength(&block_counts)) {
        return;
    }

    let first_block = block_offsets[idx];
    let count = block_counts[idx];

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
        let word_base = (first_block + block) * 16u;

        for (var t = 0u; t < 16u; t = t + 1u) {
            w[t] = block_words[word_base + t];
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

    let out = idx * 8u;
    output_words[out] = h0;
    output_words[out + 1u] = h1;
    output_words[out + 2u] = h2;
    output_words[out + 3u] = h3;
    output_words[out + 4u] = h4;
    output_words[out + 5u] = h5;
    output_words[out + 6u] = h6;
    output_words[out + 7u] = h7;
}
"#;

#[cfg(test)]
mod tests {
    use super::*;
    use sha2::{Digest, Sha256};

    #[test]
    fn sha256_padding_handles_single_block_messages() {
        let mut words = Vec::new();
        append_padded_sha256_blocks(&[1u8; 55], &mut words).unwrap();

        assert_eq!(words.len(), 16);
    }

    #[test]
    fn sha256_padding_handles_multi_block_messages() {
        let mut words = Vec::new();
        append_padded_sha256_blocks(&[1u8; 120], &mut words).unwrap();

        assert_eq!(words.len(), 48);
    }

    #[test]
    fn sha256_kernel_matches_cpu_when_wgpu_is_available() {
        let Ok(backend) = WgpuBackend::initialize() else {
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
}
