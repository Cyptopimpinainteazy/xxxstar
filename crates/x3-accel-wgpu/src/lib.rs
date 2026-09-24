//! wgpu support crate for X3 accelerator backends.
//!
//! This crate owns device/adapter discovery for non-CUDA acceleration. Kernel
//! implementations are deliberately explicit: algorithms without a WGSL kernel
//! return an error instead of silently falling back inside the backend.

use std::sync::{mpsc, Mutex, MutexGuard};

const SHA256_WORDS: usize = 8;
const SHA256_BLOCK_WORDS: usize = 16;
/// Keccak-256 output is 4 lanes = 8 little-endian u32 words.
const KECCAK_WORDS: usize = 8;
/// Keccak-256 absorbs at 1088 bits = 136 bytes = 34 little-endian u32 words
/// per block.
const KECCAK_BLOCK_WORDS: usize = 34;
const KECCAK_RATE_BYTES: usize = KECCAK_BLOCK_WORDS * 4;
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
    keccak_bind_group_layout: wgpu::BindGroupLayout,
    keccak_pipeline: wgpu::ComputePipeline,
    keccak_buffers: Mutex<Option<KeccakBuffers>>,
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

struct KeccakBuffers {
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
        let (keccak_bind_group_layout, keccak_pipeline) = create_keccak_pipeline(&device);

        Ok(Self {
            adapter_info,
            device,
            queue,
            sha256_bind_group_layout,
            sha256_pipeline,
            sha256_buffers: Mutex::new(None),
            keccak_bind_group_layout,
            keccak_pipeline,
            keccak_buffers: Mutex::new(None),
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

    /// Keccak-256 compute kernel entrypoint (Ethereum's legacy-padded variant).
    ///
    /// Same buffer discipline as `sha256_batch`: one thread per message, one
    /// dispatch for the whole batch, single readback. Before this existed the
    /// backend returned `KernelUnavailable` for keccak256, so every EVM-facing
    /// hash batch fell back to CPU even with a working device.
    pub fn keccak256_batch(&self, inputs: &[Vec<u8>]) -> Result<Vec<[u8; 32]>, WgpuAccelError> {
        if inputs.is_empty() {
            return Ok(Vec::new());
        }

        let mut block_words = Vec::new();
        let mut block_offsets = Vec::with_capacity(inputs.len());
        let mut block_counts = Vec::with_capacity(inputs.len());
        for input in inputs {
            block_offsets.push(
                u32::try_from(block_words.len() / KECCAK_BLOCK_WORDS)
                    .map_err(|_| WgpuAccelError::InvalidInput("block offset exceeds u32"))?,
            );
            let before = block_words.len();
            append_padded_keccak_blocks(input, &mut block_words)?;
            block_counts.push(
                u32::try_from((block_words.len() - before) / KECCAK_BLOCK_WORDS)
                    .map_err(|_| WgpuAccelError::InvalidInput("block count exceeds u32"))?,
            );
        }

        let output_words = inputs.len() * KECCAK_WORDS;
        let output_size = (output_words * std::mem::size_of::<u32>()) as wgpu::BufferAddress;
        let buffers_guard = self.keccak_buffers(block_words.len(), inputs.len(), output_words)?;
        let buffers = buffers_guard
            .as_ref()
            .expect("keccak buffers initialized after capacity check");

        self.queue
            .write_buffer(&buffers.blocks_buffer, 0, bytemuck::cast_slice(&block_words));
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
                label: Some("x3-keccak-encoder"),
            });
        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("x3-keccak-compute-pass"),
                timestamp_writes: None,
            });
            compute_pass.set_pipeline(&self.keccak_pipeline);
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
        for chunk in words.chunks_exact(KECCAK_WORDS) {
            let mut output = [0u8; 32];
            for (index, word) in chunk.iter().enumerate() {
                output[index * 4..index * 4 + 4].copy_from_slice(&word.to_le_bytes());
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

    fn keccak_buffers(
        &self,
        required_block_words: usize,
        required_messages: usize,
        required_output_words: usize,
    ) -> Result<MutexGuard<'_, Option<KeccakBuffers>>, WgpuAccelError> {
        let mut guard = self
            .keccak_buffers
            .lock()
            .map_err(|_| WgpuAccelError::BufferMapFailed("keccak buffer lock poisoned".into()))?;

        let needs_create = guard
            .as_ref()
            .map(|buffers| {
                buffers.block_word_capacity < required_block_words
                    || buffers.message_capacity < required_messages
                    || buffers.output_word_capacity < required_output_words
            })
            .unwrap_or(true);

        if needs_create {
            let block_word_capacity = required_block_words.max(34).next_power_of_two();
            let message_capacity = required_messages.max(1).next_power_of_two();
            let output_word_capacity = required_output_words.max(8).next_power_of_two();

            *guard = Some(create_keccak_buffers(
                &self.device,
                &self.keccak_bind_group_layout,
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

/// Pad with Keccak's `pad10*1` rule and emit the absorb rate as little-endian
/// words.
///
/// Keccak-256 differs from SHA3-256 only in the domain byte: Ethereum's variant
/// appends `0x01` where SHA3-256 appends `0x06`. Getting that byte wrong yields
/// a hash that looks plausible and is wrong for every Ethereum reader, which is
/// why the parity test below pins it against `keccak-hash`.
fn append_padded_keccak_blocks(
    input: &[u8],
    output_words: &mut Vec<u32>,
) -> Result<(), WgpuAccelError> {
    let mut padded = Vec::with_capacity(input.len() + KECCAK_RATE_BYTES + 1);
    padded.extend_from_slice(input);
    padded.push(0x01);
    while padded.len() % KECCAK_RATE_BYTES != 0 {
        padded.push(0);
    }
    // The final byte of the final block carries the trailing bit of pad10*1.
    let last = padded.len() - 1;
    padded[last] |= 0x80;

    if padded.len() / KECCAK_RATE_BYTES > u32::MAX as usize {
        return Err(WgpuAccelError::InvalidInput(
            "padded input block count exceeds u32",
        ));
    }

    for block in padded.chunks_exact(KECCAK_RATE_BYTES) {
        for word in block.chunks_exact(4) {
            output_words.push(u32::from_le_bytes([word[0], word[1], word[2], word[3]]));
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

fn create_keccak_pipeline(device: &wgpu::Device) -> (wgpu::BindGroupLayout, wgpu::ComputePipeline) {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("x3-keccak-wgsl"),
        source: wgpu::ShaderSource::Wgsl(KECCAK_WGSL.into()),
    });
    let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("x3-keccak-bind-group-layout"),
        entries: &[
            storage_entry(0, true),
            storage_entry(1, true),
            storage_entry(2, true),
            storage_entry(3, false),
        ],
    });
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("x3-keccak-pipeline-layout"),
        bind_group_layouts: &[&bind_group_layout],
        push_constant_ranges: &[],
    });
    let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some("x3-keccak-pipeline"),
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

fn create_keccak_buffers(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    block_word_capacity: usize,
    message_capacity: usize,
    output_word_capacity: usize,
) -> KeccakBuffers {
    let blocks_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("x3-keccak-blocks"),
        size: bytes_for_words(block_word_capacity),
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let block_offsets_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("x3-keccak-block-offsets"),
        size: bytes_for_words(message_capacity),
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let block_counts_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("x3-keccak-block-counts"),
        size: bytes_for_words(message_capacity),
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let output_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("x3-keccak-output"),
        size: bytes_for_words(output_word_capacity),
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    });
    let readback_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("x3-keccak-readback"),
        size: bytes_for_words(output_word_capacity),
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("x3-keccak-bind-group"),
        layout,
        entries: &[
            bind_entry(0, &blocks_buffer),
            bind_entry(1, &block_offsets_buffer),
            bind_entry(2, &block_counts_buffer),
            bind_entry(3, &output_buffer),
        ],
    });

    KeccakBuffers {
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
@group(0) @binding(0) var<storage, read> block_words: array<u32>;
@group(0) @binding(1) var<storage, read> block_offsets: array<u32>;
@group(0) @binding(2) var<storage, read> block_counts: array<u32>;
@group(0) @binding(3) var<storage, read_write> output_words: array<u32>;

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
fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let idx = global_id.x;
    if (idx >= arrayLength(&block_counts)) {
        return;
    }

    let first_block = block_offsets[idx];
    let count = block_counts[idx];

    var lo: array<u32, 25>;
    var hi: array<u32, 25>;
    for (var lane = 0u; lane < 25u; lane = lane + 1u) {
        lo[lane] = 0u;
        hi[lane] = 0u;
    }

    for (var block = 0u; block < count; block = block + 1u) {
        let word_base = (first_block + block) * 34u;
        for (var lane = 0u; lane < 17u; lane = lane + 1u) {
            lo[lane] = lo[lane] ^ block_words[word_base + 2u * lane];
            hi[lane] = hi[lane] ^ block_words[word_base + 2u * lane + 1u];
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

    #[test]
    fn keccak_padding_places_domain_and_final_bits_little_endian() {
        // Keccak's `pad10*1` for an empty message is a single block whose first
        // byte is the domain byte `0x01` and whose last byte carries the
        // trailing bit. Words are little-endian, so those land in word 0 and
        // the top byte of the final word.
        let mut words = Vec::new();
        append_padded_keccak_blocks(&[], &mut words).unwrap();

        assert_eq!(words.len(), KECCAK_BLOCK_WORDS);
        assert_eq!(words[0], 0x0000_0001);
        assert_eq!(words[KECCAK_BLOCK_WORDS - 1], 0x8000_0000);
    }

    #[test]
    fn keccak_padding_keeps_a_full_rate_block_intact() {
        // 135 bytes leaves exactly one byte for the domain byte in a single
        // block; 136 bytes must spill the trailing bit into a second block.
        let mut exact = Vec::new();
        append_padded_keccak_blocks(&[0x11u8; 135], &mut exact).unwrap();
        assert_eq!(exact.len(), KECCAK_BLOCK_WORDS);

        let mut spill = Vec::new();
        append_padded_keccak_blocks(&[0x11u8; 136], &mut spill).unwrap();
        assert_eq!(spill.len(), KECCAK_BLOCK_WORDS * 2);
    }

    #[test]
    fn keccak256_kernel_matches_cpu_when_wgpu_is_available() {
        let Ok(backend) = WgpuBackend::initialize() else {
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
