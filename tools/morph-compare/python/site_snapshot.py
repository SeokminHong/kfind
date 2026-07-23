from __future__ import annotations


KFIND_SITE_PROFILES = (
    ("kfind-embedded-any", "embedded", "any"),
    ("kfind-embedded-smart", "embedded", "smart"),
    ("kfind-full-pos-any", "full-pos", "any"),
    ("kfind-full-pos-smart", "full-pos", "smart"),
)


def profile_comparison_summary(
    evaluation: dict[str, object],
    external_performance: dict[str, object],
) -> dict[str, object]:
    boundary_comparison = evaluation["boundary_comparison"]
    boundary_profiles = boundary_comparison["profiles"]
    quality = {}
    performance = {}
    profiles = []

    for site_profile, resource_profile, boundary in KFIND_SITE_PROFILES:
        result = boundary_profiles[resource_profile][boundary]
        profiles.append(site_profile)
        quality[site_profile] = {
            "overall": result["quality"],
            "contract_adjusted": {
                "overall": result["contract_adjusted_quality"]
            },
        }
        performance[site_profile] = result["performance"]

    for backend in evaluation["backends"]:
        if backend.startswith("kfind-"):
            continue
        profiles.append(backend)
        quality[backend] = evaluation["quality"][backend]
        performance[backend] = external_performance[backend]

    return {
        "profiles": profiles,
        "quality": quality,
        "performance": performance,
    }


def site_profile_comparisons(report: dict[str, object]) -> dict[str, object]:
    canonical = report
    query_matrix = report["query_matrix"]["explicit_pos"]
    robustness = report["robustness"]["explicit_pos"]
    return {
        "canonical": profile_comparison_summary(
            canonical, canonical["external_baselines"]["performance"]
        ),
        "query_matrix": profile_comparison_summary(
            query_matrix, query_matrix["external_baselines"]["performance"]
        ),
        "robustness": profile_comparison_summary(
            robustness, robustness["performance"]
        ),
    }
