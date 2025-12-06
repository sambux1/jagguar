import math
import json
import estimator
from estimator import LWE

# parameters
key_length_list  = [2048, 3072]
q_list           = [2**127 - 1, 2**127 - 1]
p_list           = [2**64, 2**92]
adv_samples_list = [sage.all.oo, sage.all.oo]

results = []

# iterate through the zipped parameters
for key_length, q, p, adv_samples in zip(key_length_list, q_list, p_list, adv_samples_list):
    # uniform distribution over Z_q
    secret_distribution = estimator.nd.UniformMod(q)

    # LWR -> LWE reduction: model rounding as noise with variance (q/p)^2 / 12
    # equivalent Gaussian sigma ≈ (q/p)/sqrt(12), hence alpha = sigma / q = 1/(p*sqrt(12))
    alpha = 1.0 / (float(p) * math.sqrt(12.0))
    error_distribution = estimator.nd.DiscreteGaussianAlpha(alpha, q)

    # set LWE parameters
    params = LWE.Parameters(
        n=key_length,
        q=q,
        m=adv_samples,
        Xs=secret_distribution,
        Xe=error_distribution
    )

    # compute min security (bits) across attacks with minimal handling
    estimates = LWE.estimate(params)
    best_attack, best_bits = min(
        ((name, math.log2(float(res["rop"]))) for name, res in estimates.items()),
        key=lambda x: x[1]
    )
    min_bits_int = int(math.floor(best_bits))
    # pretty-print p as a power of 2 when applicable
    p_is_pow2 = (p > 0) and ((p & (p - 1)) == 0)
    p_exp = p.bit_length() - 1 if p_is_pow2 else None
    p_str = f"2^{p_exp} (={p})" if p_exp is not None else str(p)
    print(f"[params n={key_length}, q={q}, p={p_str}] min_security_bits={min_bits_int}")

    # collect result for JSON output
    try:
        m_serializable = int(adv_samples)
    except Exception:
        m_serializable = str(adv_samples)
    results.append({
        "n": int(key_length),
        "q": int(q),
        "p": p_str,
        "m": m_serializable,
        "min_security_bits": min_bits_int
    })

# write all results to JSON
with open("shprg_parameters.json", "w") as f:
    json.dump(results, f, indent=2)