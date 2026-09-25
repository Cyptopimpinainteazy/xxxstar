#include <cuda_runtime.h>
#include <stdint.h>
#include <string.h>
#include <time.h>
#include <sys/mman.h>
#include <sys/stat.h>
#include <fcntl.h>
#include <unistd.h>

#include "secp256k1.cuh"  // for BigInt + modular math helpers

/* Shared-memory ring buffer for committed atomic swaps.
 * The X3 runtime monitors /x3_atomic_commits to detect finalized swaps. */
#define X3_COMMIT_SHM_NAME  "/x3_atomic_commits"
#define X3_COMMIT_RING_SIZE 256

typedef struct {
    uint8_t  svm_prefix[32];       /* first 32 bytes of SVM entry (contains swap_id) */
    uint8_t  evm_prefix[32];       /* first 32 bytes of EVM entry                    */
    uint64_t committed_at_ns;      /* CLOCK_REALTIME nanoseconds                     */
    volatile uint8_t valid;        /* set last; reader checks this with ACQUIRE       */
    uint8_t  _pad[7];
} AtomicCommitEntry;

typedef struct {
    volatile uint32_t write_idx;   /* monotonic counter, mod ring size for slot       */
    uint32_t          _pad[7];     /* separate from data on a cache line              */
    AtomicCommitEntry entries[X3_COMMIT_RING_SIZE];
} AtomicCommitRing;

extern "C" {

/*
 * Atomic swap verification kernel.
 *
 * This host-side helper leverages existing GPU batch verifiers for each signature
 * scheme, then enforces the atomic invariant: both sides must validate.
 *
 * Layouts (per entry):
 *   SVM: [sig (64) | pubkey (32) | msg (32)] = 128 bytes
 *   EVM: [sig (65) | pubkey (64) | msg (32)] = 161 bytes
 *
 * For EVM, the last byte of the signature is ignored (recovery id).
 */

#define SVM_ENTRY_SIZE 128
#define EVM_ENTRY_SIZE 161

extern int ed25519_verify_batch_host(const unsigned char *entries, int count, unsigned char *results);
extern int secp256k1_ecdsa_verify_host(
    const unsigned char* u1_bytes,
    const unsigned char* u2_bytes,
    const unsigned char* pubkey_bytes,
    int count,
    unsigned char* out_x
);

static inline void bytes_to_bigint_be(const unsigned char *src, BigInt &out) {
    for (int i = 0; i < 8; i++) {
        int offset = 28 - i * 4;
        out.data[i] = (uint32_t)src[offset] << 24 |
                      (uint32_t)src[offset + 1] << 16 |
                      (uint32_t)src[offset + 2] << 8 |
                      (uint32_t)src[offset + 3];
    }
}

static inline void bigint_to_bytes_be(const BigInt &in, unsigned char *dst) {
    for (int i = 0; i < 8; i++) {
        int offset = 28 - i * 4;
        dst[offset]     = (unsigned char)((in.data[i] >> 24) & 0xFF);
        dst[offset + 1] = (unsigned char)((in.data[i] >> 16) & 0xFF);
        dst[offset + 2] = (unsigned char)((in.data[i] >> 8) & 0xFF);
        dst[offset + 3] = (unsigned char)(in.data[i] & 0xFF);
    }
}

static void compute_u1_u2_r(
    const unsigned char *sig,
    const unsigned char *msg,
    BigInt &u1,
    BigInt &u2,
    BigInt &r_out,
    bool &valid
) {
    // Curve order N (secp256k1)
    const BigInt N = {
        {0xD0364141u, 0xBFD25E8D, 0xAF48A03B, 0xBAAEDCE6,
         0xFFFFFFFEu, 0xFFFFFFFFu, 0xFFFFFFFFu, 0xFFFFFFFFu}
    };

    // r = sig[0..31], s = sig[32..63]
    BigInt r, s;
    bytes_to_bigint_be(sig, r);
    bytes_to_bigint_be(sig + 32, s);
    copy_bigint(r_out, r);

    if (is_zero(r) || is_zero(s) || compare_bigint(r, N) >= 0 || compare_bigint(s, N) >= 0) {
        valid = false;
        init_bigint(u1, 0);
        init_bigint(u2, 0);
        return;
    }

    BigInt z;
    bytes_to_bigint_be(msg, z);

    BigInt w;
    mod_inverse(w, s, N);

    mul_mod(u1, z, w, N);
    mul_mod(u2, r, w, N);

    valid = true;
}

int atomic_verify_host(
    const uint8_t* svm_data,
    const uint8_t* evm_data,
    int batch_size,
    uint8_t* status
) {
    if (batch_size <= 0) return -1;

    // 1) Verify SVM (Ed25519) side via GPU kernel
    unsigned char *svm_results = (unsigned char*)malloc(batch_size);
    if (!svm_results) return -1;

    int svm_rc = ed25519_verify_batch_host(svm_data, batch_size, svm_results);
    if (svm_rc != 0) {
        memset(status, 0, (size_t)batch_size);
        free(svm_results);
        return 0;
    }

    // 2) Prepare inputs for secp256k1 GPU verifier
    unsigned char *u1_bytes = (unsigned char*)malloc(batch_size * 32);
    unsigned char *u2_bytes = (unsigned char*)malloc(batch_size * 32);
    unsigned char *r_bytes = (unsigned char*)malloc(batch_size * 32);
    unsigned char *pubkeys = (unsigned char*)malloc(batch_size * 64);
    unsigned char *out_x = (unsigned char*)malloc(batch_size * 32);

    if (!u1_bytes || !u2_bytes || !r_bytes || !pubkeys || !out_x) {
        free(svm_results);
        free(u1_bytes);
        free(u2_bytes);
        free(r_bytes);
        free(pubkeys);
        free(out_x);
        return -1;
    }

    for (int i = 0; i < batch_size; i++) {
        const unsigned char *entry = evm_data + (size_t)i * EVM_ENTRY_SIZE;
        const unsigned char *sig = entry;
        const unsigned char *pk = entry + 65;
        const unsigned char *msg = entry + 65 + 64;

        // Copy pubkey (expect 64-byte uncompressed)
        memcpy(pubkeys + (size_t)i * 64, pk, 64);

        BigInt u1, u2, r;
        bool valid_sig;
        compute_u1_u2_r(sig, msg, u1, u2, r, valid_sig);

        if (!valid_sig) {
            // invalid signatures map to zero-valued inputs (GPU will likely return 0)
            init_bigint(u1, 0);
            init_bigint(u2, 0);
            init_bigint(r, 0);
        }

        bigint_to_bytes_be(u1, u1_bytes + (size_t)i * 32);
        bigint_to_bytes_be(u2, u2_bytes + (size_t)i * 32);
        bigint_to_bytes_be(r, r_bytes + (size_t)i * 32);
    }

    int evm_rc = secp256k1_ecdsa_verify_host(
        u1_bytes, u2_bytes, pubkeys, batch_size, out_x
    );

    if (evm_rc != 0) {
        memset(status, 0, (size_t)batch_size);
    } else {
        for (int i = 0; i < batch_size; i++) {
            bool svm_ok = svm_results[i] != 0;
            bool evm_ok = true;
            // Compare out_x to r
            if (memcmp(out_x + (size_t)i * 32, r_bytes + (size_t)i * 32, 32) != 0) {
                evm_ok = false;
            }
            status[i] = (svm_ok && evm_ok) ? 1 : 0;
        }
    }

    free(svm_results);
    free(u1_bytes);
    free(u2_bytes);
    free(r_bytes);
    free(pubkeys);
    free(out_x);

    return 0;
}

int atomic_commit_host(
    const uint8_t* svm_data,
    const uint8_t* evm_data,
    int batch_size
) {
    if (!svm_data || !evm_data || batch_size <= 0) return 0;

    /* Open (or create) the POSIX shm ring buffer. */
    int fd = shm_open(X3_COMMIT_SHM_NAME, O_CREAT | O_RDWR, 0600);
    if (fd < 0) return 0;

    const size_t shm_size = sizeof(AtomicCommitRing);
    if (ftruncate(fd, (off_t)shm_size) < 0) {
        close(fd);
        return 0;
    }

    AtomicCommitRing* ring = (AtomicCommitRing*)mmap(
        NULL, shm_size, PROT_READ | PROT_WRITE, MAP_SHARED, fd, 0
    );
    close(fd);
    if (ring == MAP_FAILED) return 0;

    int committed = 0;
    for (int i = 0; i < batch_size; i++) {
        /* Atomically claim the next ring slot. */
        uint32_t slot = __atomic_fetch_add(&ring->write_idx, 1u, __ATOMIC_SEQ_CST)
                        % X3_COMMIT_RING_SIZE;
        AtomicCommitEntry* e = &ring->entries[slot];

        const size_t svm_copy = SVM_ENTRY_SIZE < 32 ? SVM_ENTRY_SIZE : 32;
        const size_t evm_copy = EVM_ENTRY_SIZE < 32 ? EVM_ENTRY_SIZE : 32;
        memcpy(e->svm_prefix, svm_data + (size_t)i * SVM_ENTRY_SIZE, svm_copy);
        memcpy(e->evm_prefix, evm_data + (size_t)i * EVM_ENTRY_SIZE, evm_copy);

        struct timespec ts;
        clock_gettime(CLOCK_REALTIME, &ts);
        e->committed_at_ns = (uint64_t)ts.tv_sec * 1000000000ULL
                           + (uint64_t)ts.tv_nsec;

        /* Release store: make the entry visible to readers. */
        __atomic_store_n(&e->valid, 1u, __ATOMIC_RELEASE);
        committed++;
    }

    munmap(ring, shm_size);
    return committed;
}

}
