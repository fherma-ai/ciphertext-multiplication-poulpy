# Ciphertext multiplication on Poulpy

An answer to `ciphertext-multiplication/ckks@1.0.0` with Poulpy 0.9.0. One
directory per point; the envelope builds the module from `config.jsonc` and
never reads the point.

0.9.0 reworked the HAL: owned keys are `AlignedBuf` rather than `Vec<u8>`,
`EncryptionLayout` is gone — the sigma rides on `Noise::ENCRYPTION` and the
encryption call no longer takes one — and `poulpy-cpu-ref` is
`poulpy-cpu-portable`. The dependency is the `v0.9.0` tag rather than a
crates.io release, so the build is not `--offline`. The 0.8.3 rows on the
boards keep their own commit, which is the one before this.

The measured call is one library call:

```rust
env.module.ckks_mul_into(&mut out, &cts[0], &cts[1], &env.tensor_key, &mut scratch)
```

It is all three stages the specification names. The result comes back at
`min(Δa, Δb)` rather than at Δ²: in Poulpy the rescale is a shift of the
torus and it happens inside the multiplication, so there is no second call.

## The parameters, which are not a prime chain

Poulpy is not RNS. There is no modulus chain and no level count: a ciphertext
is a torus of `log_delta + log_budget` bits, every multiplication spends
`log_delta` of the budget, and a point's depth is that many spends.

| | `solution-s32768-d10` | `solution-s4096-d20` |
| --- | --- | --- |
| `ring_degree` | 65536 | 65536 |
| slots the ring offers | 32768 | 32768, of which 4096 are packed |
| `log_delta` | 50 | 50 |
| `log_budget` | 500 | 1000 |
| torus width | 550 bits | 1050 bits |
| `base2k` / `dsize` | 19 / 1 | 19 / 1 |

Both points land on a ring of 65536 for the same two reasons the RNS answers
do: the wide one because 32768 slots is half a ring of 65536, the deep one
because 1050 bits of torus will not fit a ring of 32768, which carries 881 at
128 bits classic.

The security floor is enforced by the envelope, not by this configuration: a
ring that cannot carry the declared width at 128 bits classic refuses to seal
before anything is measured, which is the same guarantee OpenFHE gives and
DESILO does not.

## The backend, and why it is most of the number

The backend is a cargo feature in `Cargo.toml`, which is the author's file;
`config.jsonc` cannot name one, because a backend is a type and it
parameterises every layout in the envelope. Each directory here names one:

| directory suffix | backend | `base2k` | limbs at 550 / 1050 bits |
| --- | --- | --- | --- |
| *(none)* | `fft64-avx`, the architecture's default | 19 | 29 / 56 |
| `-ntt4x30-avx512` | exact NTT, four 30-bit primes, AVX-512 | 52 | 11 / 21 |
| `-ntt3x42-ifma` | exact NTT, three 42-bit primes, IFMA | 52 | 11 / 21 |
| `-ntt3x42-ifma-rayon` | the same, over a rayon pool | 52 | 11 / 21 |

The limb ceiling belongs to the backend: `2·base2k + log₂(n)` has to stay
inside the transform's exact range, which is 53 bits of mantissa for the
approximate f64 FFT and the CRT width for the exact NTTs. So the FFT64 row
carries 29 limbs for a torus the RNS libraries carry in 11, and every
operation walks the limbs. At `base2k` 52 the limb counts are equal — 11 and
21, the same as OpenFHE's — and what is left to differ is the arithmetic
itself.

## What differs from the other rows, and is not in the point

The secret is sparse — `secret_hamming_weight` 192, the library's default
regime — where the OpenFHE and FIDESlib rows use a uniform ternary secret.
The point names a security level and not a secret distribution, and the
standard table the envelope checks against is the ternary one, so this row's
128 bits rests on an assumption the other rows do not need. It is the first
thing to change if the comparison is to be exact, and the panel note says so.
