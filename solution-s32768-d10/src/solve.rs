//! One CKKS ciphertext multiplication on Poulpy.
//!
//! `ckks_mul_into` is the whole answer, and it is the three stages the
//! specification names in one call: it tensors, it key-switches with the
//! tensor key, and the result comes back at `min(Δa, Δb)` rather than at
//! Δ² — in Poulpy the rescale is a shift of the torus and it happens inside
//! the multiplication, so there is no second call to make.
//!
//! Poulpy states the parameters differently from the RNS libraries answering
//! this point. There is no prime chain and no level count: a ciphertext is a
//! torus of `log_delta + log_budget` bits, each multiplication spends
//! `log_delta` of the budget, and the point's depth is therefore
//! `depth × log_delta` bits of budget. The envelope refuses to seal a ring
//! that cannot carry that width at 128 bits classic, which is the same
//! check OpenFHE makes and DESILO does not.
use poulpy_ckks::api::CKKSMulOps;
use poulpy_ckks::test_suite::helpers::alloc_ct;
use poulpy_hal::api::ScratchOwnedBorrow;

use crate::envelope::{Ct, Env, Pt, PtOut};
use crate::fherma::{Inputs, Outputs, Point, Tensor};

#[derive(Default)]
pub struct State;

/// Not measured. It says what was built, because the torus width and the
/// secret's weight are what this row has to be read against.
pub fn init(p: &Point, env: &mut Env) -> State {
    println!(
        "ring {} ({} slots), base2k {}, k {} bits, hamming weight {}, dsize {}",
        env.params.n,
        env.slots(),
        env.params.base2k,
        env.params.k,
        env.params.hw,
        env.params.dsize,
    );
    println!(
        "point: slots {} depth {} security {} precision {}",
        p.slots, p.depth, p.security, p.precision
    );
    State
}

/// Slot i holds element i, zero-padded to the ring's slot count — the same
/// layout as every other answer to this point.
pub fn encoding(env: &mut Env, inp: &Inputs) -> Vec<Pt> {
    vec![env.encode_reals(&inp.a.data), env.encode_reals(&inp.b.data)]
}

/// Measured, and only this.
pub fn run(_state: &mut State, env: &mut Env, cts: Vec<Ct>) -> Vec<Ct> {
    let mut out = alloc_ct(&env.params, &env.module, env.params.k);
    env.module
        .ckks_mul_into(
            &mut out,
            &cts[0],
            &cts[1],
            &env.tensor_key,
            &mut env.scratch.borrow(),
        )
        .expect("ckks_mul_into");
    vec![out]
}

pub fn decoding(p: &Point, env: &mut Env, pts: Vec<PtOut>) -> Outputs {
    let values = env.decode_reals(&pts[0], p.slots as usize);
    Outputs {
        c: Tensor { shape: vec![p.slots as i64], data: values },
    }
}

pub fn free(state: State) {
    drop(state);
}
