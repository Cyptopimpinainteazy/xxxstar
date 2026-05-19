/*
 * CUDA Stream Pipeline — Overlapped Transfer + Compute
 *
 * Implements 3-stage pipelining:
 *   Stage 1: H2D memcpy (cudaMemcpyAsync from pinned host memory)
 *   Stage 2: Kernel execution
 *   Stage 3: D2H memcpy (cudaMemcpyAsync to pinned host memory)
 *
 * By splitting work into N chunks and using N CUDA streams, we overlap
 * the memcpy of chunk[i] with the compute of chunk[i-1] and the D2H
 * of chunk[i-2], keeping the GPU memory bus and SMs simultaneously busy.
 *
 * For SHA-256 batch: latency reduces ~40% at 1M+ hashes due to overlap.
 * For PoH chains: less benefit (chains are compute-bound, small transfer).
 * For Ed25519: moderate benefit on large batches (128 bytes/sig).
 *
 * Also provides:
 *   - Persistent pinned-memory pool (avoids repeated cudaMallocHost)
 *   - Multi-GPU pipeline (each GPU gets its own stream set)
 *   - Timing/profiling via CUDA events
 *
 * Build:
 *   nvcc -arch=sm_61 -O2 -shared -Xcompiler -fPIC --use_fast_math \
 *        -lineinfo -maxrregcount=64 stream_pipeline.cu -o build/libstream_pipeline.so
 *
 * Target: sm_61 (GTX 1070), 3× GPUs
 */

#include <cuda_runtime.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

/* ───── Config ───── */
#define MAX_STREAMS     8
#define MAX_GPUS        8
#define DEFAULT_STREAMS 4
#define DEFAULT_THREADS 256

/* ───── Import SHA-256 kernel (we reuse the transform) ───── */

/* SHA-256 round constants */
__constant__ uint32_t k_pipeline_sha256[64] = {
    0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5,
    0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5,
    0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3,
    0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
    0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc,
    0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
    0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
    0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
    0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13,
    0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85,
    0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3,
    0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
    0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5,
    0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
    0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208,
    0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2
};

/* ───── SHA-256 device functions (inlined copy to avoid linking issues) ───── */

__device__ __forceinline__ uint32_t sp_rotr32(uint32_t x, int n) {
    return (x >> n) | (x << (32 - n));
}

__device__ __forceinline__ uint32_t sp_Ch(uint32_t x, uint32_t y, uint32_t z) {
    return (x & y) ^ (~x & z);
}

__device__ __forceinline__ uint32_t sp_Maj(uint32_t x, uint32_t y, uint32_t z) {
    return (x & y) ^ (x & z) ^ (y & z);
}

__device__ __forceinline__ uint32_t sp_Sigma0(uint32_t x) {
    return sp_rotr32(x, 2) ^ sp_rotr32(x, 13) ^ sp_rotr32(x, 22);
}

__device__ __forceinline__ uint32_t sp_Sigma1(uint32_t x) {
    return sp_rotr32(x, 6) ^ sp_rotr32(x, 11) ^ sp_rotr32(x, 25);
}

__device__ __forceinline__ uint32_t sp_sigma0(uint32_t x) {
    return sp_rotr32(x, 7) ^ sp_rotr32(x, 18) ^ (x >> 3);
}

__device__ __forceinline__ uint32_t sp_sigma1(uint32_t x) {
    return sp_rotr32(x, 17) ^ sp_rotr32(x, 19) ^ (x >> 10);
}

__device__ void sp_sha256_transform_32(const unsigned char *input, unsigned char *output) {
    uint32_t h0 = 0x6a09e667, h1 = 0xbb67ae85;
    uint32_t h2 = 0x3c6ef372, h3 = 0xa54ff53a;
    uint32_t h4 = 0x510e527f, h5 = 0x9b05688c;
    uint32_t h6 = 0x1f83d9ab, h7 = 0x5be0cd19;

    uint32_t W[64];

    #pragma unroll
    for (int i = 0; i < 8; i++) {
        const unsigned char *p = input + i * 4;
        W[i] = ((uint32_t)p[0] << 24) | ((uint32_t)p[1] << 16) |
               ((uint32_t)p[2] <<  8) | ((uint32_t)p[3]);
    }
    W[8] = 0x80000000u;
    #pragma unroll
    for (int i = 9; i < 15; i++) W[i] = 0;
    W[15] = 256;

    #pragma unroll
    for (int i = 16; i < 64; i++) {
        W[i] = sp_sigma1(W[i-2]) + W[i-7] + sp_sigma0(W[i-15]) + W[i-16];
    }

    uint32_t a = h0, b = h1, c = h2, d = h3;
    uint32_t e = h4, f = h5, g = h6, hh = h7;

    #pragma unroll
    for (int i = 0; i < 64; i++) {
        uint32_t T1 = hh + sp_Sigma1(e) + sp_Ch(e, f, g) + k_pipeline_sha256[i] + W[i];
        uint32_t T2 = sp_Sigma0(a) + sp_Maj(a, b, c);
        hh = g; g = f; f = e; e = d + T1;
        d = c; c = b; b = a; a = T1 + T2;
    }

    h0 += a; h1 += b; h2 += c; h3 += d;
    h4 += e; h5 += f; h6 += g; h7 += hh;

    uint32_t hs[8] = {h0, h1, h2, h3, h4, h5, h6, h7};
    #pragma unroll
    for (int i = 0; i < 8; i++) {
        output[i*4 + 0] = (unsigned char)(hs[i] >> 24);
        output[i*4 + 1] = (unsigned char)(hs[i] >> 16);
        output[i*4 + 2] = (unsigned char)(hs[i] >>  8);
        output[i*4 + 3] = (unsigned char)(hs[i]);
    }
}

/* ───── Kernels ───── */

__global__ void sha256_batch_pipeline_kernel(
    const unsigned char* __restrict__ inputs,
    int count,
    unsigned char* __restrict__ outputs
) {
    int idx = blockIdx.x * blockDim.x + threadIdx.x;
    if (idx >= count) return;
    sp_sha256_transform_32(inputs + (size_t)idx * 32, outputs + (size_t)idx * 32);
}

__global__ void sha256_poh_chain_pipeline_kernel(
    const unsigned char* __restrict__ seeds,
    int num_chains,
    int chain_length,
    unsigned char* __restrict__ results
) {
    int idx = blockIdx.x * blockDim.x + threadIdx.x;
    if (idx >= num_chains) return;

    unsigned char current[32];
    unsigned char next[32];

    const unsigned char *seed = seeds + (size_t)idx * 32;
    #pragma unroll
    for (int i = 0; i < 32; i++) current[i] = seed[i];

    for (int step = 0; step < chain_length; step++) {
        sp_sha256_transform_32(current, next);
        #pragma unroll
        for (int i = 0; i < 32; i++) current[i] = next[i];
    }

    unsigned char *out = results + (size_t)idx * 32;
    #pragma unroll
    for (int i = 0; i < 32; i++) out[i] = current[i];
}

/* ═══════════════════════════════════════════════════════════
 * Pinned Memory Pool
 * ═══════════════════════════════════════════════════════════ */

struct PinnedPool {
    unsigned char *h_in;    /* pinned input buffer  */
    unsigned char *h_out;   /* pinned output buffer  */
    size_t capacity;        /* current capacity in bytes */
    int initialized;
};

static PinnedPool g_pool = {nullptr, nullptr, 0, 0};

static int pool_ensure(size_t bytes) {
    if (g_pool.initialized && g_pool.capacity >= bytes) return 0;

    /* Free old buffers */
    if (g_pool.h_in)  cudaFreeHost(g_pool.h_in);
    if (g_pool.h_out) cudaFreeHost(g_pool.h_out);

    /* Round up to 4 MB granularity */
    size_t alloc = ((bytes + (4 << 20) - 1) / (4 << 20)) * (4 << 20);

    cudaError_t err;
    err = cudaMallocHost(&g_pool.h_in, alloc);
    if (err != cudaSuccess) {
        fprintf(stderr, "pool: cudaMallocHost input failed: %s\n", cudaGetErrorString(err));
        g_pool.initialized = 0;
        return -1;
    }
    err = cudaMallocHost(&g_pool.h_out, alloc);
    if (err != cudaSuccess) {
        cudaFreeHost(g_pool.h_in);
        g_pool.h_in = nullptr;
        fprintf(stderr, "pool: cudaMallocHost output failed: %s\n", cudaGetErrorString(err));
        g_pool.initialized = 0;
        return -1;
    }

    g_pool.capacity = alloc;
    g_pool.initialized = 1;
    return 0;
}

/* ═══════════════════════════════════════════════════════════
 * Stream-Pipelined SHA-256 Batch
 *
 * Splits `count` hashes into `num_streams` chunks.
 * Each chunk's H2D, kernel, D2H use a separate CUDA stream,
 * enabling triple-buffered overlap:
 *
 *   Stream 0: [H2D  chunk0] [Kernel chunk0] [D2H  chunk0]
 *   Stream 1:                [H2D  chunk1]   [Kernel chunk1] [D2H chunk1]
 *   Stream 2:                                 [H2D  chunk2]  [Kernel] [D2H]
 *   ...
 *
 * On GTX 1070, the PCIe bus and SMs can overlap, so this
 * hides transfer latency behind compute.
 * ═══════════════════════════════════════════════════════════ */

extern "C" int sha256_batch_streamed(
    const unsigned char *inputs,
    int count,
    unsigned char *outputs,
    int num_streams
) {
    if (count <= 0) return 0;
    if (num_streams <= 0 || num_streams > MAX_STREAMS) num_streams = DEFAULT_STREAMS;

    /* For very small batches, skip overhead */
    if (count < 1024) {
        /* Direct path: pageable memcpy, single kernel */
        unsigned char *d_in = nullptr, *d_out = nullptr;
        size_t bytes = (size_t)count * 32;
        cudaMalloc(&d_in, bytes);
        cudaMalloc(&d_out, bytes);
        cudaMemcpy(d_in, inputs, bytes, cudaMemcpyHostToDevice);
        int blocks = (count + DEFAULT_THREADS - 1) / DEFAULT_THREADS;
        sha256_batch_pipeline_kernel<<<blocks, DEFAULT_THREADS>>>(d_in, count, d_out);
        cudaDeviceSynchronize();
        cudaMemcpy(outputs, d_out, bytes, cudaMemcpyDeviceToHost);
        cudaFree(d_in);
        cudaFree(d_out);
        return 0;
    }

    size_t total_bytes = (size_t)count * 32;

    /* Allocate pinned host memory */
    if (pool_ensure(total_bytes) != 0) {
        fprintf(stderr, "sha256_batch_streamed: pinned pool alloc failed\n");
        return -1;
    }

    /* Copy input into pinned buffer */
    memcpy(g_pool.h_in, inputs, total_bytes);

    /* Allocate device memory (input + output) */
    unsigned char *d_in = nullptr, *d_out = nullptr;
    cudaMalloc(&d_in, total_bytes);
    cudaMalloc(&d_out, total_bytes);

    /* Create streams */
    cudaStream_t streams[MAX_STREAMS];
    for (int i = 0; i < num_streams; i++) {
        cudaStreamCreate(&streams[i]);
    }

    /* Divide work into chunks */
    int chunk_size = (count + num_streams - 1) / num_streams;

    for (int s = 0; s < num_streams; s++) {
        int start = s * chunk_size;
        int chunk = chunk_size;
        if (start + chunk > count) chunk = count - start;
        if (chunk <= 0) continue;

        size_t offset = (size_t)start * 32;
        size_t bytes  = (size_t)chunk * 32;

        /* Async H2D */
        cudaMemcpyAsync(d_in + offset, g_pool.h_in + offset,
                         bytes, cudaMemcpyHostToDevice, streams[s]);

        /* Kernel launch on this stream */
        int blocks = (chunk + DEFAULT_THREADS - 1) / DEFAULT_THREADS;
        sha256_batch_pipeline_kernel<<<blocks, DEFAULT_THREADS, 0, streams[s]>>>(
            d_in + offset, chunk, d_out + offset);

        /* Async D2H */
        cudaMemcpyAsync(g_pool.h_out + offset, d_out + offset,
                         bytes, cudaMemcpyDeviceToHost, streams[s]);
    }

    /* Synchronize all streams */
    for (int i = 0; i < num_streams; i++) {
        cudaStreamSynchronize(streams[i]);
        cudaStreamDestroy(streams[i]);
    }

    /* Copy result from pinned buffer to caller buffer */
    memcpy(outputs, g_pool.h_out, total_bytes);

    cudaFree(d_in);
    cudaFree(d_out);
    return 0;
}

/* ═══════════════════════════════════════════════════════════
 * Stream-Pipelined PoH Chain (multi-stream)
 *
 * Each CUDA stream handles a subset of chains. Since chains are
 * compute-bound (sequential hashing), the main benefit is
 * overlapping H2D/D2H with compute of other stream chunks.
 * ═══════════════════════════════════════════════════════════ */

extern "C" int sha256_poh_chain_streamed(
    const unsigned char *seeds,
    int num_chains,
    int chain_length,
    unsigned char *results,
    int num_streams
) {
    if (num_chains <= 0 || chain_length <= 0) return 0;
    if (num_streams <= 0 || num_streams > MAX_STREAMS) num_streams = DEFAULT_STREAMS;

    size_t total_bytes = (size_t)num_chains * 32;

    if (pool_ensure(total_bytes) != 0) return -1;
    memcpy(g_pool.h_in, seeds, total_bytes);

    unsigned char *d_seeds = nullptr, *d_results = nullptr;
    cudaMalloc(&d_seeds, total_bytes);
    cudaMalloc(&d_results, total_bytes);

    cudaStream_t streams[MAX_STREAMS];
    for (int i = 0; i < num_streams; i++) {
        cudaStreamCreate(&streams[i]);
    }

    int chunk_size = (num_chains + num_streams - 1) / num_streams;

    for (int s = 0; s < num_streams; s++) {
        int start = s * chunk_size;
        int chunk = chunk_size;
        if (start + chunk > num_chains) chunk = num_chains - start;
        if (chunk <= 0) continue;

        size_t offset = (size_t)start * 32;
        size_t bytes  = (size_t)chunk * 32;

        cudaMemcpyAsync(d_seeds + offset, g_pool.h_in + offset,
                         bytes, cudaMemcpyHostToDevice, streams[s]);

        int blocks = (chunk + 128 - 1) / 128;
        sha256_poh_chain_pipeline_kernel<<<blocks, 128, 0, streams[s]>>>(
            d_seeds + offset, chunk, chain_length, d_results + offset);

        cudaMemcpyAsync(g_pool.h_out + offset, d_results + offset,
                         bytes, cudaMemcpyDeviceToHost, streams[s]);
    }

    for (int i = 0; i < num_streams; i++) {
        cudaStreamSynchronize(streams[i]);
        cudaStreamDestroy(streams[i]);
    }

    memcpy(results, g_pool.h_out, total_bytes);

    cudaFree(d_seeds);
    cudaFree(d_results);
    return 0;
}

/* ═══════════════════════════════════════════════════════════
 * Multi-GPU Stream Pipeline
 *
 * Each GPU gets its own set of CUDA streams. Work is distributed
 * across GPUs, then each GPU uses stream pipelining internally.
 * ═══════════════════════════════════════════════════════════ */

extern "C" int sha256_batch_multi_gpu_streamed(
    const unsigned char *inputs,
    int count,
    unsigned char *outputs,
    int streams_per_gpu
) {
    if (count <= 0) return 0;
    if (streams_per_gpu <= 0) streams_per_gpu = DEFAULT_STREAMS;

    int device_count = 0;
    cudaGetDeviceCount(&device_count);
    if (device_count <= 0) return -1;

    if (device_count == 1) {
        cudaSetDevice(0);
        return sha256_batch_streamed(inputs, count, outputs, streams_per_gpu);
    }

    /* Per-GPU pinned host buffers and device buffers */
    int per_gpu = (count + device_count - 1) / device_count;

    /* Allocate one big pinned input & output buffer */
    size_t total_bytes = (size_t)count * 32;
    unsigned char *h_pin_in = nullptr, *h_pin_out = nullptr;
    cudaMallocHost(&h_pin_in, total_bytes);
    cudaMallocHost(&h_pin_out, total_bytes);
    memcpy(h_pin_in, inputs, total_bytes);

    for (int dev = 0; dev < device_count; dev++) {
        int start = dev * per_gpu;
        int chunk = per_gpu;
        if (start + chunk > count) chunk = count - start;
        if (chunk <= 0) continue;

        cudaSetDevice(dev);

        size_t offset = (size_t)start * 32;
        size_t bytes  = (size_t)chunk * 32;

        unsigned char *d_in = nullptr, *d_out = nullptr;
        cudaMalloc(&d_in, bytes);
        cudaMalloc(&d_out, bytes);

        /* Create streams for this GPU */
        int ns = streams_per_gpu;
        if (ns > MAX_STREAMS) ns = MAX_STREAMS;
        cudaStream_t streams[MAX_STREAMS];
        for (int i = 0; i < ns; i++) cudaStreamCreate(&streams[i]);

        /* Pipeline within this GPU's chunk */
        int sub_chunk = (chunk + ns - 1) / ns;
        for (int s = 0; s < ns; s++) {
            int ss = s * sub_chunk;
            int sc = sub_chunk;
            if (ss + sc > chunk) sc = chunk - ss;
            if (sc <= 0) continue;

            size_t so = (size_t)ss * 32;
            size_t sb = (size_t)sc * 32;

            cudaMemcpyAsync(d_in + so, h_pin_in + offset + so,
                             sb, cudaMemcpyHostToDevice, streams[s]);

            int blocks = (sc + DEFAULT_THREADS - 1) / DEFAULT_THREADS;
            sha256_batch_pipeline_kernel<<<blocks, DEFAULT_THREADS, 0, streams[s]>>>(
                d_in + so, sc, d_out + so);

            cudaMemcpyAsync(h_pin_out + offset + so, d_out + so,
                             sb, cudaMemcpyDeviceToHost, streams[s]);
        }

        for (int i = 0; i < ns; i++) {
            cudaStreamSynchronize(streams[i]);
            cudaStreamDestroy(streams[i]);
        }

        cudaFree(d_in);
        cudaFree(d_out);
    }

    memcpy(outputs, h_pin_out, total_bytes);
    cudaFreeHost(h_pin_in);
    cudaFreeHost(h_pin_out);

    cudaSetDevice(0);
    return 0;
}

/* ═══════════════════════════════════════════════════════════
 * Benchmark / Profiling Helper
 *
 * Returns timing breakdown: H2D, compute, D2H, total (in ms)
 * Uses CUDA events for precise GPU timing.
 * ═══════════════════════════════════════════════════════════ */

struct PipelineTimings {
    float total_ms;
    float h2d_ms;
    float compute_ms;
    float d2h_ms;
    float throughput_mhps; /* million hashes per second */
};

extern "C" int sha256_pipeline_benchmark(
    int count,
    int num_streams,
    float *total_ms,
    float *h2d_ms,
    float *compute_ms,
    float *d2h_ms,
    float *throughput_mhps
) {
    if (count <= 0) return -1;
    if (num_streams <= 0) num_streams = DEFAULT_STREAMS;

    size_t bytes = (size_t)count * 32;

    /* Pinned host buffers */
    unsigned char *h_in = nullptr, *h_out = nullptr;
    cudaMallocHost(&h_in, bytes);
    cudaMallocHost(&h_out, bytes);

    /* Fill with deterministic data */
    for (size_t i = 0; i < bytes; i++) h_in[i] = (unsigned char)(i & 0xFF);

    /* Device buffers */
    unsigned char *d_in = nullptr, *d_out = nullptr;
    cudaMalloc(&d_in, bytes);
    cudaMalloc(&d_out, bytes);

    /* CUDA events for timing */
    cudaEvent_t ev_start, ev_h2d_done, ev_compute_done, ev_d2h_done;
    cudaEventCreate(&ev_start);
    cudaEventCreate(&ev_h2d_done);
    cudaEventCreate(&ev_compute_done);
    cudaEventCreate(&ev_d2h_done);

    /* Warmup */
    cudaMemcpy(d_in, h_in, bytes, cudaMemcpyHostToDevice);
    int blocks = (count + DEFAULT_THREADS - 1) / DEFAULT_THREADS;
    sha256_batch_pipeline_kernel<<<blocks, DEFAULT_THREADS>>>(d_in, count, d_out);
    cudaDeviceSynchronize();

    /* Timed non-pipelined run (for comparison) */
    cudaEventRecord(ev_start);
    cudaMemcpy(d_in, h_in, bytes, cudaMemcpyHostToDevice);
    cudaEventRecord(ev_h2d_done);
    sha256_batch_pipeline_kernel<<<blocks, DEFAULT_THREADS>>>(d_in, count, d_out);
    cudaEventRecord(ev_compute_done);
    cudaMemcpy(h_out, d_out, bytes, cudaMemcpyDeviceToHost);
    cudaEventRecord(ev_d2h_done);
    cudaEventSynchronize(ev_d2h_done);

    float t_total, t_h2d, t_compute, t_d2h;
    cudaEventElapsedTime(&t_h2d, ev_start, ev_h2d_done);
    cudaEventElapsedTime(&t_compute, ev_h2d_done, ev_compute_done);
    cudaEventElapsedTime(&t_d2h, ev_compute_done, ev_d2h_done);
    cudaEventElapsedTime(&t_total, ev_start, ev_d2h_done);

    float non_pipelined_total = t_total;

    /* Now do the pipelined run */
    cudaEvent_t ev_pipe_start, ev_pipe_end;
    cudaEventCreate(&ev_pipe_start);
    cudaEventCreate(&ev_pipe_end);

    cudaStream_t streams[MAX_STREAMS];
    for (int i = 0; i < num_streams; i++) cudaStreamCreate(&streams[i]);

    int chunk_size = (count + num_streams - 1) / num_streams;

    cudaEventRecord(ev_pipe_start);

    for (int s = 0; s < num_streams; s++) {
        int start = s * chunk_size;
        int chunk = chunk_size;
        if (start + chunk > count) chunk = count - start;
        if (chunk <= 0) continue;

        size_t off = (size_t)start * 32;
        size_t cb  = (size_t)chunk * 32;

        cudaMemcpyAsync(d_in + off, h_in + off, cb, cudaMemcpyHostToDevice, streams[s]);
        int bl = (chunk + DEFAULT_THREADS - 1) / DEFAULT_THREADS;
        sha256_batch_pipeline_kernel<<<bl, DEFAULT_THREADS, 0, streams[s]>>>(
            d_in + off, chunk, d_out + off);
        cudaMemcpyAsync(h_out + off, d_out + off, cb, cudaMemcpyDeviceToHost, streams[s]);
    }

    cudaEventRecord(ev_pipe_end);
    for (int i = 0; i < num_streams; i++) {
        cudaStreamSynchronize(streams[i]);
        cudaStreamDestroy(streams[i]);
    }
    cudaEventSynchronize(ev_pipe_end);

    float pipelined_total;
    cudaEventElapsedTime(&pipelined_total, ev_pipe_start, ev_pipe_end);

    /* Output results */
    if (total_ms)         *total_ms = pipelined_total;
    if (h2d_ms)           *h2d_ms = t_h2d;
    if (compute_ms)       *compute_ms = t_compute;
    if (d2h_ms)           *d2h_ms = t_d2h;
    if (throughput_mhps)  *throughput_mhps = (float)count / (pipelined_total * 1000.0f);

    printf("=== SHA-256 Pipeline Benchmark ===\n");
    printf("  Count:             %d hashes\n", count);
    printf("  Streams:           %d\n", num_streams);
    printf("  --- Non-pipelined ---\n");
    printf("    H2D:             %.3f ms\n", t_h2d);
    printf("    Compute:         %.3f ms\n", t_compute);
    printf("    D2H:             %.3f ms\n", t_d2h);
    printf("    Total:           %.3f ms\n", non_pipelined_total);
    printf("    Throughput:      %.2f M hashes/s\n",
           (float)count / (non_pipelined_total * 1000.0f));
    printf("  --- Pipelined (%d streams) ---\n", num_streams);
    printf("    Total:           %.3f ms\n", pipelined_total);
    printf("    Throughput:      %.2f M hashes/s\n",
           (float)count / (pipelined_total * 1000.0f));
    printf("    Speedup:         %.2fx\n",
           non_pipelined_total / pipelined_total);
    printf("================================\n");

    /* Cleanup */
    cudaEventDestroy(ev_start);
    cudaEventDestroy(ev_h2d_done);
    cudaEventDestroy(ev_compute_done);
    cudaEventDestroy(ev_d2h_done);
    cudaEventDestroy(ev_pipe_start);
    cudaEventDestroy(ev_pipe_end);
    cudaFree(d_in);
    cudaFree(d_out);
    cudaFreeHost(h_in);
    cudaFreeHost(h_out);

    return 0;
}

/* ═══════════════════════════════════════════════════════════
 * Pool cleanup (call at shutdown)
 * ═══════════════════════════════════════════════════════════ */

extern "C" void pipeline_cleanup(void) {
    if (g_pool.h_in)  { cudaFreeHost(g_pool.h_in);  g_pool.h_in = nullptr; }
    if (g_pool.h_out) { cudaFreeHost(g_pool.h_out); g_pool.h_out = nullptr; }
    g_pool.capacity = 0;
    g_pool.initialized = 0;
}

/* ═══════════════════════════════════════════════════════════
 * GPU Info
 * ═══════════════════════════════════════════════════════════ */

extern "C" void pipeline_print_info(void) {
    int device_count = 0;
    cudaGetDeviceCount(&device_count);
    printf("=== Stream Pipeline GPU Info ===\n");
    printf("  GPU count: %d\n", device_count);

    for (int i = 0; i < device_count; i++) {
        cudaDeviceProp prop;
        cudaGetDeviceProperties(&prop, i);
        printf("  GPU %d: %s\n", i, prop.name);
        printf("    Compute:      sm_%d%d\n", prop.major, prop.minor);
        printf("    SMs:          %d\n", prop.multiProcessorCount);
        printf("    Memory:       %zu MB\n", prop.totalGlobalMem / (1024*1024));
        printf("    Clock:        %d MHz\n", prop.clockRate / 1000);
        printf("    Mem Clock:    %d MHz\n", prop.memoryClockRate / 1000);
        printf("    Bus Width:    %d bit\n", prop.memoryBusWidth);
        printf("    Async Engines: %d\n", prop.asyncEngineCount);
        printf("    Concurrent Kernels: %s\n",
               prop.concurrentKernels ? "yes" : "no");
        printf("    Can overlap H2D+compute: %s\n",
               prop.asyncEngineCount >= 1 ? "yes" : "no");
        printf("    Can overlap H2D+D2H:     %s\n",
               prop.asyncEngineCount >= 2 ? "yes (dual-copy)" : "no");
    }
    printf("================================\n");
}
