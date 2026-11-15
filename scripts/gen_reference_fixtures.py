#!/usr/bin/env python3
"""Offline Reference Fixture Oracle Generator for `algebra-engine` / `cat-core`.

Generates `tests/fixtures/reference_oracles.json` with deterministic exact ground-truth
vectors for:
  - Riemann / rank-N tensor symmetry canonicalization
  - Lyndon words over {a, b}
  - Stirling numbers of the second kind S(n, k) and Bell numbers
  - Laplace, Z, and Fourier transform pairs + numerical evaluation points
  - Ito's lemma & Colombeau delta-product identities
"""

import json
import math
from pathlib import Path


def generate_lyndon_words(alphabet: str, max_len: int) -> list[str]:
    """Duval's / Chen-Fox-Lyndon lexicographic generation up to `max_len`."""
    words: list[str] = []
    n = len(alphabet)
    w = [-1]
    while w:
        w[-1] += 1
        m = len(w)
        if m <= max_len:
            words.append("".join(alphabet[i] for i in w))
        while len(w) < max_len:
            w.append(w[len(w) - m])
        while w and w[-1] == n - 1:
            w.pop()
    return words


def stirling2(n: int, k: int) -> int:
    if n == 0 and k == 0:
        return 1
    if n == 0 or k == 0 or k > n:
        return 0
    dp = [[0] * (k + 1) for _ in range(n + 1)]
    dp[0][0] = 1
    for i in range(1, n + 1):
        for j in range(1, min(i, k) + 1):
            dp[i][j] = j * dp[i - 1][j] + dp[i - 1][j - 1]
    return dp[n][k]


def build_fixtures() -> dict:
    lyndon_ab_4 = generate_lyndon_words("ab", 4)
    stirling_table = [
        {"n": n, "k": k, "value": stirling2(n, k)}
        for n in range(1, 7)
        for k in range(1, n + 1)
    ]
    bell_numbers = [
        {"n": n, "value": sum(stirling2(n, k) for k in range(n + 1))}
        for n in range(0, 7)
    ]

    riemann_oracles = [
        {
            "name": "riemann_antisym_pair_01",
            "perm": [1, 0, 2, 3],
            "expected_canonical": [0, 1, 2, 3],
            "expected_sign": -1,
        },
        {
            "name": "riemann_pair_swap",
            "perm": [2, 3, 0, 1],
            "expected_canonical": [0, 1, 2, 3],
            "expected_sign": 1,
        },
        {
            "name": "riemann_both_pairs_antisym",
            "perm": [3, 2, 1, 0],
            "expected_canonical": [0, 1, 2, 3],
            "expected_sign": 1,
        },
    ]

    transform_oracles = [
        {
            "kind": "laplace",
            "input": "exp(-2*t)",
            "s_sample": 3.0,
            "expected_value": 1.0 / (3.0 + 2.0),
        },
        {
            "kind": "z_transform",
            "input": "0.5^n",
            "z_sample": 2.0,
            "expected_value": 2.0 / (2.0 - 0.5),
        },
        {
            "kind": "fourier",
            "input": "exp(-t^2)",
            "omega_sample": 2.0,
            "expected_value": math.sqrt(math.pi) * math.exp(-(2.0**2) / 4.0),
        },
    ]

    ito_colombeau_oracles = {
        "ito_brownian_square": {
            "f": "W^2",
            "drift_coeff": 1.0,
            "diffusion_coeff_expr": "2*W",
        },
        "colombeau_heaviside_sq_deriv_at_half": {
            "x": 0.5,
            "expected_product_coeff": 1.0,
        },
    }

    return {
        "schema_version": 1,
        "lyndon_ab_len4": lyndon_ab_4,
        "stirling_second_kind": stirling_table,
        "bell_numbers": bell_numbers,
        "riemann_contractions": riemann_oracles,
        "transforms": transform_oracles,
        "ito_colombeau": ito_colombeau_oracles,
    }


def main() -> None:
    root = Path(__file__).resolve().parent.parent
    out_path = root / "tests" / "fixtures" / "reference_oracles.json"
    out_path.parent.mkdir(parents=True, exist_ok=True)
    fixtures = build_fixtures()
    out_path.write_text(json.dumps(fixtures, indent=2) + "\n", encoding="utf-8")
    print(f"Wrote {out_path}")


if __name__ == "__main__":
    main()
