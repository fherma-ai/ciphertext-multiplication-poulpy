# Ciphertext multiplication on Poulpy

An answer to `ciphertext-multiplication/ckks@1.0.0` with Poulpy 0.8.3. One
directory per point; the envelope builds the module from `config.jsonc` and
never reads the point.

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

## What differs from the other rows, and is not in the point

The secret is sparse — `secret_hamming_weight` 192, the library's default
regime — where the OpenFHE and FIDESlib rows use a uniform ternary secret.
The point names a security level and not a secret distribution, and the
standard table the envelope checks against is the ternary one, so this row's
128 bits rests on an assumption the other rows do not need. It is the first
thing to change if the comparison is to be exact, and the panel note says so.
