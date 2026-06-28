"""发布版本——知道 semver 规则和 bump 逻辑。"""

from dataclasses import dataclass


@dataclass
class ReleaseVersion:
    """发布版本号——领域值对象。"""

    major: int
    minor: int
    patch: int

    @classmethod
    def parse(cls, s: str) -> "ReleaseVersion":
        parts = s.strip().split(".")
        if len(parts) != 3:
            raise ValueError(f"版本号格式错误: {s} (需要 X.Y.Z)")
        return cls(int(parts[0]), int(parts[1]), int(parts[2]))

    def bump(self, level: str) -> "ReleaseVersion":
        if level == "major":
            return ReleaseVersion(self.major + 1, 0, 0)
        elif level == "minor":
            return ReleaseVersion(self.major, self.minor + 1, 0)
        else:
            return ReleaseVersion(self.major, self.minor, self.patch + 1)

    def __str__(self) -> str:
        return f"{self.major}.{self.minor}.{self.patch}"
