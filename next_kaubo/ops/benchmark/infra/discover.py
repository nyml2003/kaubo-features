from pathlib import Path
from domain.model import Case


def discover_cases(suites_dir: Path) -> list[Case]:
    """Find all benchmark case directories that contain at least one main.* file."""
    cases: list[Case] = []
    if not suites_dir.is_dir():
        return cases
    for d in sorted(suites_dir.iterdir()):
        if d.is_dir() and list(d.glob("main.*")):
            cases.append(Case(name=d.name, path=d))
    return cases
