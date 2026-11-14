//! Amitsur-Levitzki matrix polynomial identity zero-tester over finite fields.
//!
//! By the Amitsur-Levitzki Theorem (1950), the standard polynomial:
//! S_{2m}(A_1, \dots, A_{2m}) = \sum_{\sigma \in S_{2m}} \text{sgn}(\sigma) A_{\sigma(1)} \cdots A_{\sigma(2m)}
//! vanishes identically for all m \times m matrices over any commutative ring,
//! but does NOT vanish for degree < 2m.

/// 2x2 matrix over finite field F_p.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct Matrix2Fp {
    pub p: u64,
    pub data: [u64; 4], // [a, b, c, d] row-major
}

impl Matrix2Fp {
    pub fn new(p: u64, a: u64, b: u64, c: u64, d: u64) -> Self {
        Self {
            p,
            data: [a % p, b % p, c % p, d % p],
        }
    }

    pub fn zero(p: u64) -> Self {
        Self { p, data: [0; 4] }
    }

    pub fn is_zero(&self) -> bool {
        self.data == [0; 4]
    }

    pub fn add(&self, rhs: &Self) -> Self {
        let p = self.p;
        Self {
            p,
            data: [
                (self.data[0] + rhs.data[0]) % p,
                (self.data[1] + rhs.data[1]) % p,
                (self.data[2] + rhs.data[2]) % p,
                (self.data[3] + rhs.data[3]) % p,
            ],
        }
    }

    pub fn sub(&self, rhs: &Self) -> Self {
        let p = self.p;
        Self {
            p,
            data: [
                (self.data[0] + p - (rhs.data[0] % p)) % p,
                (self.data[1] + p - (rhs.data[1] % p)) % p,
                (self.data[2] + p - (rhs.data[2] % p)) % p,
                (self.data[3] + p - (rhs.data[3] % p)) % p,
            ],
        }
    }

    pub fn mul(&self, rhs: &Self) -> Self {
        let p = self.p;
        let (a1, b1, c1, d1) = (self.data[0], self.data[1], self.data[2], self.data[3]);
        let (a2, b2, c2, d2) = (rhs.data[0], rhs.data[1], rhs.data[2], rhs.data[3]);
        Self {
            p,
            data: [
                (a1 * a2 + b1 * c2) % p,
                (a1 * b2 + b1 * d2) % p,
                (c1 * a2 + d1 * c2) % p,
                (c1 * b2 + d1 * d2) % p,
            ],
        }
    }
}

/// Amitsur-Levitzki zero-tester.
pub struct AmitsurLevitzki;

impl AmitsurLevitzki {
    /// Compute the standard polynomial S_4 on four 2x2 matrices:
    /// S_4(A, B, C, D) = \sum_{\sigma \in S_4} sgn(\sigma) X_{\sigma(1)} X_{\sigma(2)} X_{\sigma(3)} X_{\sigma(4)}.
    /// By Amitsur-Levitzki, this MUST be identically 0 for any 4 matrices of size 2x2!
    pub fn standard_polynomial_s4(
        m: &[Matrix2Fp; 4],
    ) -> Matrix2Fp {
        let p = m[0].p;
        let mut sum = Matrix2Fp::zero(p);

        // Generate all 24 permutations of (0, 1, 2, 3) with parity
        let mut perm = [0usize, 1, 2, 3];
        Self::permute_s4(&mut perm, 0, &mut |p_arr, sign| {
            let prod = m[p_arr[0]]
                .mul(&m[p_arr[1]])
                .mul(&m[p_arr[2]])
                .mul(&m[p_arr[3]]);
            if sign > 0 {
                sum = sum.add(&prod);
            } else {
                sum = sum.sub(&prod);
            }
        });

        sum
    }

    fn permute_s4<F: FnMut(&[usize; 4], i8)>(arr: &mut [usize; 4], idx: usize, cb: &mut F) {
        if idx == 4 {
            let sign = Self::permutation_sign(arr);
            cb(arr, sign);
            return;
        }
        for i in idx..4 {
            arr.swap(idx, i);
            Self::permute_s4(arr, idx + 1, cb);
            arr.swap(idx, i);
        }
    }

    fn permutation_sign(arr: &[usize; 4]) -> i8 {
        let mut inversions = 0;
        for i in 0..4 {
            for j in (i + 1)..4 {
                if arr[i] > arr[j] {
                    inversions += 1;
                }
            }
        }
        if inversions % 2 == 1 { -1 } else { 1 }
    }
}
