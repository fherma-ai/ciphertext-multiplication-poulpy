// GENERATED for ciphertext_multiplication/ckks@1.0.0. Do not edit — `--update` rewrites it.
//
// The envelope: everything cryptographic that is not the answer. It reads
// config.jsonc, builds the module, generates the keys, and encrypts and
// decrypts at the boundary. The secret key lives in this file and is not
// reachable from solve.rs: the four functions receive `Env`, and `Env` does
// not carry it.
//
// The security floor is enforced here, not trusted to the author: a ring
// that cannot carry the declared torus width at 128 bits classic refuses to
// seal, before anything is measured.
use std::collections::HashMap;
use std::fs;

use poulpy_ckks::api::{CKKSPlaintextVecOps, CKKSEncryptOps, CKKSDecryptOps};
use poulpy_ckks::layouts::{CKKSCiphertextOwned, CKKSModuleAlloc, CKKSPlaintextOwned};
use poulpy_ckks::test_suite::helpers::{
    alloc_ct, alloc_scratch, ckks_decrypt_with_prec, gen_atk, gen_sk_with_raw, gen_tsk, upload_pt,
};
use poulpy_ckks::test_suite::reference_encoder::ReferenceEncoder;
use poulpy_ckks::test_suite::CKKSTestParams;
use poulpy_ckks::{CKKSInfos, CKKSLayout, CKKSMeta, SetCKKSInfos, SlotsKind};
use poulpy_core::EncryptionLayout;
use poulpy_core::layouts::{
    Base2K, Degree, GLWEAutomorphismKeyPrepared, GLWELayout,
    GLWESecretPrepared, GLWETensorKeyPrepared, LWEInfos, Rank, TorusPrecision,
};
use poulpy_hal::api::ScratchOwnedBorrow;
use poulpy_hal::layouts::{GaloisElement as _, HostBytesBackend, Module, ScratchOwned};
use poulpy_hal::source::Source;

// The backend, chosen at build time by a cargo feature in Cargo.toml — which
// is yours, and which the harness never overwrites. It cannot come from
// config.jsonc: the backend is a type and it parameterises every layout in
// this file, so a run-time name could not name it.
//
//   (none)                 the architecture's default, which is what a first
//                          build gets anywhere
//   fft64-avx              approximate f64 FFT              base2k ≤ 19
//   fft64-avx-rayon
//   fft64-avx512
//   fft64-avx512-rayon
//   ntt4x30-avx            exact NTT, four 30-bit primes    base2k ≤ 52
//   ntt4x30-avx-rayon
//   ntt4x30-avx512
//   ntt4x30-avx512-rayon
//   ntt3x42-ifma           exact NTT, three 42-bit primes   base2k ≤ 52
//   ntt3x42-ifma-rayon
//
// The limb ceiling is the backend's arithmetic and not a preference:
// `2·base2k + log2(n)` has to stay inside the transform's exact range, which
// is 53 bits of mantissa for the f64 FFT and the CRT width for the NTTs. It
// is the ceiling that decides the work: a torus of 550 bits is 29 limbs at
// base2k 19 and 11 at 52, and every operation walks the limbs.
//
// `*-rayon` backends spread one operation over a rayon pool, sized from
// `thread_count` before any key is generated.
const BACKENDS_PICKED: usize = cfg!(feature = "fft64-avx") as usize
    + cfg!(feature = "fft64-avx-rayon") as usize
    + cfg!(feature = "fft64-avx512") as usize
    + cfg!(feature = "fft64-avx512-rayon") as usize
    + cfg!(feature = "ntt4x30-avx") as usize
    + cfg!(feature = "ntt4x30-avx-rayon") as usize
    + cfg!(feature = "ntt4x30-avx512") as usize
    + cfg!(feature = "ntt4x30-avx512-rayon") as usize
    + cfg!(feature = "ntt3x42-ifma") as usize
    + cfg!(feature = "ntt3x42-ifma-rayon") as usize
    + cfg!(feature = "fft64-neon") as usize
    + cfg!(feature = "fft64-neon-rayon") as usize
    + cfg!(feature = "ntt4x30-neon") as usize
    + cfg!(feature = "ntt4x30-neon-rayon") as usize;
const _: () = assert!(
    BACKENDS_PICKED <= 1,
    "one backend feature at a time: none means the architecture's default",
);

macro_rules! chosen_backend {
    ($path:path, $name:literal, $ceiling:expr, $threaded:expr) => {
        pub use $path as Backend;
        /// What ran, for the record: the row's arithmetic is not in the config.
        pub const BACKEND: &str = $name;
        /// The widest limb this backend's transform keeps exact.
        pub const BASE2K_CEILING: usize = $ceiling;
        /// Whether one operation is spread over a rayon pool.
        pub const THREADED: bool = $threaded;
    };
}

#[cfg(all(target_arch = "x86_64", feature = "fft64-avx"))]
chosen_backend!(poulpy_cpu_avx::FFT64Avx, "fft64-avx", 19, false);
#[cfg(all(target_arch = "x86_64", feature = "fft64-avx-rayon"))]
chosen_backend!(poulpy_cpu_avx::FFT64AvxRayon, "fft64-avx-rayon", 19, true);
#[cfg(all(target_arch = "x86_64", feature = "fft64-avx512"))]
chosen_backend!(poulpy_cpu_avx512::FFT64Avx512, "fft64-avx512", 19, false);
#[cfg(all(target_arch = "x86_64", feature = "fft64-avx512-rayon"))]
chosen_backend!(poulpy_cpu_avx512::FFT64Avx512Rayon, "fft64-avx512-rayon", 19, true);
#[cfg(all(target_arch = "x86_64", feature = "ntt4x30-avx"))]
chosen_backend!(poulpy_cpu_avx::NTT4x30Avx, "ntt4x30-avx", 52, false);
#[cfg(all(target_arch = "x86_64", feature = "ntt4x30-avx-rayon"))]
chosen_backend!(poulpy_cpu_avx::NTT4x30AvxRayon, "ntt4x30-avx-rayon", 52, true);
#[cfg(all(target_arch = "x86_64", feature = "ntt4x30-avx512"))]
chosen_backend!(poulpy_cpu_avx512::NTT4x30Avx512, "ntt4x30-avx512", 52, false);
#[cfg(all(target_arch = "x86_64", feature = "ntt4x30-avx512-rayon"))]
chosen_backend!(poulpy_cpu_avx512::NTT4x30Avx512Rayon, "ntt4x30-avx512-rayon", 52, true);
#[cfg(all(target_arch = "x86_64", feature = "ntt3x42-ifma"))]
chosen_backend!(poulpy_cpu_avx512::NTT3x42Ifma, "ntt3x42-ifma", 52, false);
#[cfg(all(target_arch = "x86_64", feature = "ntt3x42-ifma-rayon"))]
chosen_backend!(poulpy_cpu_avx512::NTT3x42IfmaRayon, "ntt3x42-ifma-rayon", 52, true);

#[cfg(all(target_arch = "aarch64", feature = "fft64-neon"))]
chosen_backend!(poulpy_cpu_arm::FFT64Neon, "fft64-neon", 19, false);
#[cfg(all(target_arch = "aarch64", feature = "fft64-neon-rayon"))]
chosen_backend!(poulpy_cpu_arm::FFT64NeonRayon, "fft64-neon-rayon", 19, true);
#[cfg(all(target_arch = "aarch64", feature = "ntt4x30-neon"))]
chosen_backend!(poulpy_cpu_arm::NTT4x30Neon, "ntt4x30-neon", 52, false);
#[cfg(all(target_arch = "aarch64", feature = "ntt4x30-neon-rayon"))]
chosen_backend!(poulpy_cpu_arm::NTT4x30NeonRayon, "ntt4x30-neon-rayon", 52, true);

// No feature named: the architecture's default, so that a clone builds and
// runs before anybody has read this far.
#[cfg(all(target_arch = "x86_64", not(any(feature = "fft64-avx", feature = "fft64-avx-rayon", feature = "fft64-avx512", feature = "fft64-avx512-rayon", feature = "ntt4x30-avx", feature = "ntt4x30-avx-rayon", feature = "ntt4x30-avx512", feature = "ntt4x30-avx512-rayon", feature = "ntt3x42-ifma", feature = "ntt3x42-ifma-rayon", feature = "fft64-neon", feature = "fft64-neon-rayon", feature = "ntt4x30-neon", feature = "ntt4x30-neon-rayon"))))]
chosen_backend!(poulpy_cpu_avx::FFT64Avx, "fft64-avx", 19, false);
#[cfg(all(target_arch = "aarch64", not(any(feature = "fft64-avx", feature = "fft64-avx-rayon", feature = "fft64-avx512", feature = "fft64-avx512-rayon", feature = "ntt4x30-avx", feature = "ntt4x30-avx-rayon", feature = "ntt4x30-avx512", feature = "ntt4x30-avx512-rayon", feature = "ntt3x42-ifma", feature = "ntt3x42-ifma-rayon", feature = "fft64-neon", feature = "fft64-neon-rayon", feature = "ntt4x30-neon", feature = "ntt4x30-neon-rayon"))))]
chosen_backend!(poulpy_cpu_arm::FFT64Neon, "fft64-neon", 19, false);

/// Sizes the rayon pool for a `*-rayon` backend, and does nothing for the
/// others. Called before any key is generated, so it is never measured.
fn set_threads(threads: usize) {
    #[cfg(feature = "rayon")]
    {
        let wanted = if threads == 0 {
            std::thread::available_parallelism().map(|n| n.get()).unwrap_or(1)
        } else {
            threads
        };
        // Built once per process; a second call is the error this ignores.
        let _ = rayon::ThreadPoolBuilder::new().num_threads(wanted).build_global();
    }
    #[cfg(not(feature = "rayon"))]
    let _ = threads;
}

pub type Encoder = ReferenceEncoder<poulpy_cpu_ref::FFT64ReimTable<f64>>;
/// A packing on its way in: what encoding produces and encryption consumes.
pub type Pt = CKKSPlaintextOwned<Backend>;
/// A packing on its way out: what decryption produces and decoding reads.
pub type PtOut = CKKSPlaintextOwned<HostBytesBackend>;
pub type Ct = CKKSCiphertextOwned<Backend>;

/// The HE standard's table: the widest modulus a ring degree carries at
/// 128 bits classic. The envelope's floor.
const MAX_TORUS_BITS_128_CLASSIC: &[(usize, usize)] = &[
    (2048, 54),
    (4096, 109),
    (8192, 218),
    (16384, 438),
    (32768, 881),
    (65536, 1782),
];

pub struct Config {
    pub scheme: String,
    pub ring_degree: usize,
    pub base2k: usize,
    pub log_delta: usize,
    pub log_budget: usize,
    pub secret_hamming_weight: usize,
    pub dsize: usize,
    pub thread_count: usize,
    pub rotation_indexes: Vec<i64>,
    pub security: String,
}

impl Config {
    pub fn load(path: &str) -> Config {
        let raw = fs::read_to_string(path).expect("config.jsonc is unreadable");
        // JSONC: `//` to end of line is a comment.
        let stripped: String = raw
            .lines()
            .map(|line| line.split("//").next().unwrap_or(""))
            .collect::<Vec<_>>()
            .join("\n");
        let json: serde_json::Value =
            serde_json::from_str(&stripped).expect("config.jsonc is not valid JSONC");

        let number = |name: &str, fallback: u64| -> usize {
            json.get(name).and_then(|v| v.as_u64()).unwrap_or(fallback) as usize
        };
        let word = |name: &str, fallback: &str| -> String {
            json.get(name)
                .and_then(|v| v.as_str())
                .unwrap_or(fallback)
                .to_string()
        };

        Config {
            scheme: word("scheme", "ckks"),
            ring_degree: number("ring_degree", 8192),
            base2k: number("base2k", 19),
            log_delta: number("log_delta", 30),
            log_budget: number("log_budget", 122),
            secret_hamming_weight: number("secret_hamming_weight", 192),
            dsize: number("dsize", 1),
            thread_count: number("thread_count", 0),
            rotation_indexes: json
                .get("rotation_indexes")
                .and_then(|v| v.as_array())
                .map(|list| list.iter().filter_map(|v| v.as_i64()).collect())
                .unwrap_or_default(),
            security: word("security", "128_classic"),
        }
    }
}

/// Everything the four functions may hold: the module, the parameters, the
/// evaluation keys the config declared, and the scratch every operation
/// borrows. The secret key is deliberately not here.
pub struct Env {
    pub params: CKKSTestParams,
    pub module: Module<Backend>,
    pub host_module: Module<HostBytesBackend>,
    pub encoder: Encoder,
    pub tensor_key: GLWETensorKeyPrepared<Vec<u8>, Backend>,
    /// Keyed by galois element: pass the whole map to `ckks_rotate_into`.
    pub rotation_keys: HashMap<i64, GLWEAutomorphismKeyPrepared<Vec<u8>, Backend>>,
    pub scratch: ScratchOwned<Backend>,
}

/// The envelope proper: the Env the answer sees, and the secret it does not.
pub struct Envelope {
    pub env: Env,
    sk: GLWESecretPrepared<Vec<u8>, Backend>,
}

impl Envelope {
    pub fn seal(cfg: &Config) -> Envelope {
        assert_eq!(cfg.scheme, "ckks", "this envelope speaks ckks and nothing else");
        assert_eq!(cfg.security, "128_classic", "unknown security floor");
        let k = cfg.log_delta + cfg.log_budget;
        let ceiling = MAX_TORUS_BITS_128_CLASSIC
            .iter()
            .find(|(n, _)| *n == cfg.ring_degree)
            .map(|(_, bits)| *bits)
            .unwrap_or_else(|| panic!("ring_degree {} is not a standard degree", cfg.ring_degree));
        assert!(
            k <= ceiling,
            "torus width {} bits needs a larger ring: degree {} carries at most {} at 128 bits classic",
            k, cfg.ring_degree, ceiling,
        );

        assert!(
            cfg.base2k <= BASE2K_CEILING,
            "base2k {} is wider than the {} backend keeps exact ({})",
            cfg.base2k, BACKEND, BASE2K_CEILING,
        );
        // Before the keys, so the pool is outside every measurement.
        set_threads(cfg.thread_count);

        let params = CKKSTestParams {
            n: cfg.ring_degree,
            base2k: cfg.base2k,
            k,
            prec_meta: CKKSMeta {
                log_sparsity: 0,
                log_delta: cfg.log_delta,
                slots: SlotsKind::Complex,
            },
            prec_log_budget: cfg.log_budget.min(127usize.saturating_sub(cfg.log_delta)),
            hw: cfg.secret_hamming_weight,
            dsize: cfg.dsize,
            rank: 1,
        };

        let module = Module::<Backend>::new(params.n as u64);
        let host_module = Module::<HostBytesBackend>::new(params.n as u64);
        let encoder = Encoder::new(params.n / 2).expect("encoder");
        let (sk_raw, sk) = gen_sk_with_raw(&params, &module, &host_module, entropy());
        let mut scratch = alloc_scratch(&params, &module);
        let tensor_key = gen_tsk(&params, &module, &sk_raw, &mut scratch.borrow());
        let mut rotation_keys = HashMap::new();
        for &shift in &cfg.rotation_indexes {
            let galois = module.galois_element(shift);
            rotation_keys.insert(
                galois,
                gen_atk(&params, &module, galois, &sk_raw, &mut scratch.borrow()),
            );
        }
        // sk_raw drops here: only the prepared secret survives, inside the
        // envelope, and the evaluation keys are already cut.
        Envelope {
            env: Env { params, module, host_module, encoder, tensor_key, rotation_keys, scratch },
            sk,
        }
    }

    pub fn encrypt(&mut self, packings: Vec<Pt>) -> Vec<Ct> {
        let env = &mut self.env;
        let layout = GLWELayout {
            n: Degree(env.params.n as u32),
            base2k: Base2K(env.params.base2k as u32),
            k: TorusPrecision(env.params.k as u32),
            rank: Rank(1),
        };
        let enc = EncryptionLayout::new_from_default_sigma(layout).expect("encryption layout");
        packings
            .into_iter()
            .map(|pt| {
                let mut ct = alloc_ct(&env.params, &env.module, env.params.k);
                let mut xa = Source::new(entropy());
                let mut xe = Source::new(entropy());
                env.module
                    .ckks_encrypt_sk(&mut ct, &pt, &self.sk, &enc, &mut xe, &mut xa, &mut env.scratch.borrow())
                    .expect("encrypt");
                ct
            })
            .collect()
    }

    pub fn decrypt(&mut self, cts: Vec<Ct>) -> Vec<PtOut> {
        let env = &mut self.env;
        cts.iter()
            .map(|ct| {
                // Cap the headroom so the centered plaintext fits the decode
                // codec: dropping unused high-order budget is lossless.
                let log_delta = ct.log_delta();
                let log_budget = ct
                    .log_budget()
                    .min(env.params.prec().log_budget())
                    .min(127usize.saturating_sub(log_delta));
                let prec = CKKSLayout {
                    glwe_layout: GLWELayout {
                        n: ct.n(),
                        base2k: ct.base2k(),
                        k: TorusPrecision((log_delta + log_budget) as u32),
                        rank: Rank(1),
                    },
                    meta: CKKSMeta { log_sparsity: 0, log_delta, slots: SlotsKind::Complex },
                };
                ckks_decrypt_with_prec(&env.module, ct, &self.sk, prec, &mut env.scratch.borrow())
                    .expect("decrypt")
            })
            .collect()
    }
}

impl Env {
    /// The slot count: how many values one packing carries.
    pub fn slots(&self) -> usize {
        self.params.n / 2
    }

    /// Real values into one packing, zero-padded to the slot count. The
    /// imaginary lane is left empty; a layout that wants both writes its own
    /// encode against `self.encoder`.
    pub fn encode_reals(&mut self, values: &[f64]) -> Pt {
        let m = self.slots();
        assert!(values.len() <= m, "{} values into {} slots", values.len(), m);
        let mut re = values.to_vec();
        re.resize(m, 0.0);
        let im = vec![0.0f64; m];
        let prec = self.params.prec();
        let mut host_pt = self
            .host_module
            .ckks_pt_vec_alloc(Base2K(self.params.base2k as u32), prec.k());
        host_pt.set_meta(prec.meta());
        self.encoder.encode_reim(&mut host_pt, &re, &im).expect("encode");
        upload_pt(&self.module, &host_pt)
    }

    /// The real lane of a decrypted packing, first `count` slots.
    pub fn decode_reals(&mut self, pt: &PtOut, count: usize) -> Vec<f64> {
        let m = self.slots();
        let mut re = vec![0.0f64; m];
        let mut im = vec![0.0f64; m];
        self.encoder.decode_reim(pt, &mut re, &mut im).expect("decode");
        re.truncate(count);
        re
    }
}

/// Seed material from the operating system, exactly 32 bytes a call —
/// /dev/urandom is a stream without an end, so it is read, never slurped.
fn entropy() -> [u8; 32] {
    use std::io::Read;

    let mut seed = [0u8; 32];
    let filled = fs::File::open("/dev/urandom")
        .and_then(|mut source| source.read_exact(&mut seed))
        .is_ok();
    if !filled {
        // A container without /dev/urandom is broken, but a weak seed is
        // still better than a fixed one.
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default();
        seed[..16].copy_from_slice(&now.as_nanos().to_le_bytes());
    }
    seed
}
